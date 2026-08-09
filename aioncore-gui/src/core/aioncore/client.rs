use std::path::Path;
use std::sync::OnceLock;
use std::time::Duration;

use reqwest::{Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::sync::watch;

use super::events::{CoreEvent, CoreEventHub};
use super::models::{ApiResponse, AskAnswerRequest, ConfirmRequest, Confirmation, ErrorResponse, HealthResponse};
use super::websocket::{ConnectionHandle, WebSocketClient};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    #[default]
    Disconnected,
}

#[derive(Debug, thiserror::Error)]
pub enum CoreClientError {
    #[error("backend URL is invalid")]
    InvalidBackendUrl,
    #[error("request timed out")]
    Timeout,
    #[error("backend connection failed")]
    Transport,
    #[error("backend returned HTTP {status}")]
    Http {
        status: StatusCode,
        code: Option<String>,
        message: Option<String>,
        details: Option<Value>,
    },
    #[error("backend response could not be decoded")]
    Decode,
}

#[derive(Clone)]
pub struct CoreClient {
    backend_url: Url,
    http: reqwest::Client,
    events: CoreEventHub,
    status: watch::Sender<ConnectionStatus>,
}

impl CoreClient {
    pub fn new(backend_url: &str) -> Result<Self, CoreClientError> {
        let backend_url = Url::parse(backend_url).map_err(|_| CoreClientError::InvalidBackendUrl)?;
        let (status, _) = watch::channel(ConnectionStatus::Disconnected);
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .user_agent("aioncore-gui")
            .build()
            .map_err(|_| CoreClientError::Transport)?;

        Ok(Self {
            backend_url,
            http,
            events: CoreEventHub::new(),
            status,
        })
    }

    pub fn backend_url(&self) -> &Url {
        &self.backend_url
    }

    pub fn events(&self) -> &CoreEventHub {
        &self.events
    }

    pub fn connection_status(&self) -> ConnectionStatus {
        *self.status.borrow()
    }

    pub fn subscribe_connection_status(&self) -> watch::Receiver<ConnectionStatus> {
        self.status.subscribe()
    }

    pub async fn health(&self) -> Result<HealthResponse, CoreClientError> {
        self.request_json(Method::GET, "/health", None).await
    }

    pub async fn list_confirmations(&self, conversation_id: &str) -> Result<Vec<Confirmation>, CoreClientError> {
        let path = format!("/api/conversations/{conversation_id}/confirmations");
        let response: ApiResponse<Vec<Confirmation>> = self.request_json(Method::GET, &path, None).await?;
        response.data.ok_or(CoreClientError::Decode)
    }

    pub async fn confirm(
        &self,
        conversation_id: &str,
        call_id: &str,
        request: ConfirmRequest,
    ) -> Result<(), CoreClientError> {
        let path = format!("/api/conversations/{conversation_id}/confirmations/{call_id}/confirm");
        let response: ApiResponse<Value> = self
            .request_json(
                Method::POST,
                &path,
                Some(serde_json::to_value(request).map_err(|_| CoreClientError::Decode)?),
            )
            .await?;
        if response.success {
            Ok(())
        } else {
            Err(CoreClientError::Decode)
        }
    }

    pub async fn answer_ask(
        &self,
        conversation_id: &str,
        request_id: &str,
        request: AskAnswerRequest,
    ) -> Result<(), CoreClientError> {
        let path = format!("/api/conversations/{conversation_id}/asks/{request_id}/answer");
        let response: ApiResponse<Value> = self
            .request_json(
                Method::POST,
                &path,
                Some(serde_json::to_value(request).map_err(|_| CoreClientError::Decode)?),
            )
            .await?;
        if response.success {
            Ok(())
        } else {
            Err(CoreClientError::Decode)
        }
    }

    pub async fn upload_file(
        &self,
        path: &Path,
        file_name: &str,
        conversation_id: &str,
    ) -> Result<String, CoreClientError> {
        let path = path.to_path_buf();
        let file_name = file_name.to_owned();
        let conversation_id = conversation_id.to_owned();
        let client = self.http.clone();
        let url = self.endpoint("/api/fs/upload")?;

        runtime()
            .spawn(async move {
                let data = tokio::fs::read(path).await.map_err(|_| CoreClientError::Transport)?;
                let part = reqwest::multipart::Part::bytes(data).file_name(file_name.clone());
                let form = reqwest::multipart::Form::new()
                    .part("file", part)
                    .text("file_name", file_name)
                    .text("conversation_id", conversation_id);
                let response = client
                    .post(url)
                    .multipart(form)
                    .send()
                    .await
                    .map_err(|error| map_transport_error(&error))?;
                let status = response.status();
                let payload = response.text().await.map_err(|_| CoreClientError::Decode)?;
                if !status.is_success() {
                    return Err(map_http_error(status, serde_json::from_str(&payload).ok()));
                }
                let response: ApiResponse<String> =
                    serde_json::from_str(&payload).map_err(|_| CoreClientError::Decode)?;
                response.data.ok_or(CoreClientError::Decode)
            })
            .await
            .map_err(|_| CoreClientError::Transport)?
    }

    pub async fn request_json<T>(&self, method: Method, path: &str, body: Option<Value>) -> Result<T, CoreClientError>
    where
        T: DeserializeOwned + Send + 'static,
    {
        let url = self.endpoint(path)?;
        let method_name = method.as_str().to_owned();
        let path = path.to_owned();
        let client = self.http.clone();
        let events = self.events.clone();

        runtime()
            .spawn(async move {
                let mut request = client.request(method, url);
                if let Some(body) = body {
                    let body = serde_json::to_vec(&body).map_err(|_| CoreClientError::Decode)?;
                    request = request.header("content-type", "application/json").body(body);
                }

                let response = request.send().await.map_err(|error| {
                    let mapped = map_transport_error(&error);
                    record_request_failure(&events, method_name.clone(), &path, &mapped);
                    mapped
                })?;
                let status = response.status();
                let payload = response.text().await.map_err(|_| CoreClientError::Decode)?;
                if !status.is_success() {
                    let error = map_http_error(status, serde_json::from_str(&payload).ok());
                    record_request_failure(&events, method_name, &path, &error);
                    return Err(error);
                }

                serde_json::from_str(&payload).map_err(|_| CoreClientError::Decode)
            })
            .await
            .map_err(|_| CoreClientError::Transport)?
    }

    pub fn connect(&self) -> ConnectionHandle {
        WebSocketClient::new(
            self.websocket_url(),
            self.http.clone(),
            self.endpoint("/health").expect("validated backend URL"),
            self.events.clone(),
            self.status.clone(),
        )
        .start(runtime())
    }

    fn endpoint(&self, path: &str) -> Result<Url, CoreClientError> {
        self.backend_url
            .join(path)
            .map_err(|_| CoreClientError::InvalidBackendUrl)
    }

    fn websocket_url(&self) -> Url {
        let mut url = self.backend_url.clone();
        let scheme = match url.scheme() {
            "http" => "ws",
            "https" => "wss",
            _ => "ws",
        };
        let _ = url.set_scheme(scheme);
        url.set_path("/ws");
        url.set_query(None);
        url
    }
}

fn record_request_failure(events: &CoreEventHub, method: String, path: &str, error: &CoreClientError) {
    let status = match error {
        CoreClientError::Http { status, .. } => Some(status.as_u16()),
        _ => None,
    };
    log::warn!("AionCore request failed: method={method}, path={path}, status={status:?}");
    events.publish(CoreEvent::RequestFailed {
        method,
        path: path.to_owned(),
        status,
    });
}

fn runtime() -> &'static tokio::runtime::Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_all()
            .build()
            .expect("failed to create AionCore client runtime")
    })
}

fn map_transport_error(error: &reqwest::Error) -> CoreClientError {
    if error.is_timeout() {
        CoreClientError::Timeout
    } else {
        CoreClientError::Transport
    }
}

fn map_http_error(status: StatusCode, response: Option<ErrorResponse>) -> CoreClientError {
    CoreClientError::Http {
        status,
        code: response.as_ref().map(|error| error.code.clone()),
        message: response.as_ref().map(|error| error.error.clone()),
        details: response.and_then(|error| error.details),
    }
}

#[cfg(test)]
mod tests {
    use super::{ConnectionStatus, CoreClient, CoreClientError};

    #[test]
    fn converts_http_backend_url_to_websocket_url() {
        let client = CoreClient::new("http://127.0.0.1:25808/api").unwrap();

        assert_eq!(client.websocket_url().as_str(), "ws://127.0.0.1:25808/ws");
        assert_eq!(client.connection_status(), ConnectionStatus::Disconnected);
    }

    #[test]
    fn rejects_invalid_backend_url() {
        assert!(matches!(
            CoreClient::new("not a URL"),
            Err(CoreClientError::InvalidBackendUrl)
        ));
    }
}
