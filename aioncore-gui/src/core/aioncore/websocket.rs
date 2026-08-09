use std::time::Duration;

use futures::StreamExt;
use reqwest::Url;
use tokio::sync::{oneshot, watch};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;

use super::client::ConnectionStatus;
use super::events::{CoreEvent, CoreEventHub};
use super::models::WebSocketMessage;

const INITIAL_RETRY_DELAY: Duration = Duration::from_secs(1);
const MAX_RETRY_DELAY: Duration = Duration::from_secs(30);

pub struct ConnectionHandle {
    close: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl ConnectionHandle {
    pub async fn close(mut self) {
        if let Some(close) = self.close.take() {
            let _ = close.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

impl Drop for ConnectionHandle {
    fn drop(&mut self) {
        if let Some(close) = self.close.take() {
            let _ = close.send(());
        }
    }
}

pub struct WebSocketClient {
    url: Url,
    health_url: Url,
    http: reqwest::Client,
    events: CoreEventHub,
    status: watch::Sender<ConnectionStatus>,
}

impl WebSocketClient {
    pub fn new(
        url: Url,
        http: reqwest::Client,
        health_url: Url,
        events: CoreEventHub,
        status: watch::Sender<ConnectionStatus>,
    ) -> Self {
        Self {
            url,
            health_url,
            http,
            events,
            status,
        }
    }

    pub fn start(self, runtime: &tokio::runtime::Runtime) -> ConnectionHandle {
        let (close, close_receiver) = oneshot::channel();
        let task = runtime.spawn(self.run(close_receiver));
        ConnectionHandle {
            close: Some(close),
            task: Some(task),
        }
    }

    async fn run(self, mut close: oneshot::Receiver<()>) {
        let mut retry_delay = INITIAL_RETRY_DELAY;
        let mut retries = 0_u32;

        loop {
            let _ = self.status.send(ConnectionStatus::Connecting);
            match tokio_tungstenite::connect_async(self.url.as_str()).await {
                Ok((stream, _)) => {
                    if self.health().await {
                        retry_delay = INITIAL_RETRY_DELAY;
                        retries = 0;
                        let _ = self.status.send(ConnectionStatus::Connected);
                        self.events.publish(CoreEvent::Connected);
                        log::info!("AionCore WebSocket connected");

                        if self.read_until_disconnect(stream, &mut close).await {
                            break;
                        }
                    } else {
                        log::warn!("AionCore health check failed after WebSocket connection");
                    }
                }
                Err(_) => {
                    log::warn!("AionCore WebSocket connection failed after {retries} retries");
                }
            }

            let _ = self.status.send(ConnectionStatus::Disconnected);
            self.events.publish(CoreEvent::Disconnected);
            retries = retries.saturating_add(1);
            log::info!("AionCore WebSocket reconnect scheduled: retry={retries}");

            tokio::select! {
                _ = &mut close => break,
                _ = tokio::time::sleep(retry_delay) => {
                    retry_delay = retry_delay.saturating_mul(2).min(MAX_RETRY_DELAY);
                }
            }
        }

        let _ = self.status.send(ConnectionStatus::Disconnected);
    }

    async fn health(&self) -> bool {
        match self.http.get(self.health_url.clone()).send().await {
            Ok(response) if response.status().is_success() => true,
            Ok(response) => {
                log::warn!("AionCore health request failed: status={}", response.status().as_u16());
                false
            }
            Err(_) => {
                log::warn!("AionCore health request failed");
                false
            }
        }
    }

    async fn read_until_disconnect<S>(&self, stream: S, close: &mut oneshot::Receiver<()>) -> bool
    where
        S: futures::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
    {
        let mut stream = stream;
        loop {
            tokio::select! {
                _ = &mut *close => return true,
                frame = stream.next() => match frame {
                    Some(Ok(Message::Text(text))) => self.dispatch_text(&text),
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => return false,
                    Some(Ok(_)) => {}
                }
            }
        }
    }

    fn dispatch_text(&self, text: &str) {
        match serde_json::from_str::<WebSocketMessage>(text) {
            Ok(event) => {
                log::debug!("AionCore WebSocket event received: name={}", event.name);
                self.events.publish(CoreEvent::WebSocketEvent(event));
            }
            Err(_) => log::warn!("AionCore WebSocket event was malformed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WebSocketClient;
    use crate::core::aioncore::client::ConnectionStatus;
    use crate::core::aioncore::events::{CoreEvent, CoreEventHub};
    use tokio::sync::watch;

    #[tokio::test]
    async fn dispatches_valid_name_and_data_envelope() {
        let events = CoreEventHub::new();
        let mut receiver = events.subscribe();
        let mut stream_receiver = events.subscribe_websocket_event("message.stream");
        let (status, _) = watch::channel(ConnectionStatus::Disconnected);
        let client = WebSocketClient::new(
            reqwest::Url::parse("ws://127.0.0.1/ws").unwrap(),
            reqwest::Client::new(),
            reqwest::Url::parse("http://127.0.0.1/health").unwrap(),
            events,
            status,
        );

        client.dispatch_text(r#"{"name":"message.stream","data":{"id":"ignored"}}"#);

        match receiver.recv().await.unwrap() {
            CoreEvent::WebSocketEvent(event) => {
                assert_eq!(event.name, "message.stream");
                assert_eq!(event.data["id"], "ignored");
            }
            event => panic!("unexpected event: {event:?}"),
        }
        assert_eq!(stream_receiver.recv().await.unwrap()["id"], "ignored");
    }
}
