use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::Value;
use tokio::sync::broadcast;

use super::models::WebSocketMessage;

#[derive(Clone, Debug, PartialEq)]
pub enum CoreEvent {
    Connected,
    Disconnected,
    RequestFailed {
        method: String,
        path: String,
        status: Option<u16>,
    },
    WebSocketEvent(WebSocketMessage),
}

#[derive(Clone)]
pub struct CoreEventHub {
    sender: Arc<broadcast::Sender<CoreEvent>>,
    event_senders: Arc<Mutex<HashMap<String, broadcast::Sender<Value>>>>,
}

impl CoreEventHub {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self {
            sender: Arc::new(sender),
            event_senders: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CoreEvent> {
        self.sender.subscribe()
    }

    pub fn subscribe_websocket_event(&self, name: &str) -> broadcast::Receiver<Value> {
        let mut senders = self.event_senders.lock().expect("event senders mutex poisoned");
        senders
            .entry(name.to_owned())
            .or_insert_with(|| broadcast::channel(256).0)
            .subscribe()
    }

    pub fn publish(&self, event: CoreEvent) {
        if let CoreEvent::WebSocketEvent(websocket_event) = &event
            && let Some(sender) = self
                .event_senders
                .lock()
                .expect("event senders mutex poisoned")
                .get(&websocket_event.name)
        {
            let _ = sender.send(websocket_event.data.clone());
        }
        let _ = self.sender.send(event);
    }
}

impl Default for CoreEventHub {
    fn default() -> Self {
        Self::new()
    }
}
