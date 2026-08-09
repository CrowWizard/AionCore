use std::sync::{Arc, Mutex};

use reqwest::Method;
use serde_json::Value;

use super::client::{CoreClient, CoreClientError};
use super::message_reducer::{MessageStreamReducer, MessageStreamState};
use super::models::{
    ApiResponse, AskAnswerRequest, CancelConversationRequest, CancelConversationResponse, ChatFileRef, ConfirmRequest,
    Confirmation, ConversationListResponse, ConversationNameUpdatedPayload, ConversationResponse,
    CreateConversationRequest, MessageListResponse, MessageResponse, SendMessageRequest, SendMessageResponse,
    UpdateConversationRequest,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConversationState {
    pub conversations: Vec<ConversationResponse>,
    pub selected_conversation_id: Option<String>,
    pub selected_conversation: Option<ConversationResponse>,
    pub messages: Vec<MessageResponse>,
    pub confirmations: Vec<Confirmation>,
    pub is_loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConversationEventAction {
    RefreshList,
    RefreshSelected,
    Ignored,
}

#[derive(Clone)]
pub struct ConversationStore {
    client: Arc<CoreClient>,
    state: Arc<Mutex<ConversationState>>,
    message_reducer: Arc<Mutex<MessageStreamReducer>>,
}

impl ConversationStore {
    pub fn new(client: Arc<CoreClient>) -> Self {
        Self {
            client,
            state: Arc::new(Mutex::new(ConversationState::default())),
            message_reducer: Arc::new(Mutex::new(MessageStreamReducer::default())),
        }
    }

    pub fn message_snapshot(&self) -> MessageStreamState {
        self.message_reducer
            .lock()
            .expect("message reducer mutex poisoned")
            .snapshot()
    }

    pub fn snapshot(&self) -> ConversationState {
        self.state.lock().expect("conversation state mutex poisoned").clone()
    }

    pub async fn refresh_list(&self) -> Result<Vec<ConversationResponse>, CoreClientError> {
        self.set_loading(true);
        let result = async {
            let response: ApiResponse<ConversationListResponse> = self
                .client
                .request_json(Method::GET, "/api/conversations", None)
                .await?;
            let conversations = response.data.ok_or(CoreClientError::Decode)?.items;
            let mut state = self.state.lock().expect("conversation state mutex poisoned");
            state.conversations = conversations.clone();
            state.error = None;
            Ok(conversations)
        }
        .await;
        self.set_loading(false);
        result
    }

    pub async fn create(
        &self,
        name: Option<String>,
        assistant_id: String,
        workspace: std::path::PathBuf,
    ) -> Result<ConversationResponse, CoreClientError> {
        let request = CreateConversationRequest {
            name,
            assistant: super::models::AssistantConversationRequest { id: assistant_id },
            extra: serde_json::json!({ "workspace": workspace }),
        };
        let response: ApiResponse<ConversationResponse> = self
            .client
            .request_json(
                Method::POST,
                "/api/conversations",
                Some(serde_json::to_value(request).map_err(|_| CoreClientError::Decode)?),
            )
            .await?;
        let conversation = response.data.ok_or(CoreClientError::Decode)?;
        self.refresh_list().await?;
        self.load_conversation(&conversation.id).await?;
        Ok(conversation)
    }

    pub async fn delete(&self, conversation_id: &str) -> Result<(), CoreClientError> {
        let path = conversation_path(conversation_id);
        let _: ApiResponse<Value> = self.client.request_json(Method::DELETE, &path, None).await?;
        {
            let mut state = self.state.lock().expect("conversation state mutex poisoned");
            if state.selected_conversation_id.as_deref() == Some(conversation_id) {
                state.selected_conversation_id = None;
                state.selected_conversation = None;
                state.messages.clear();
            }
        }
        self.refresh_list().await?;
        Ok(())
    }

    pub async fn rename(&self, conversation_id: &str, name: String) -> Result<ConversationResponse, CoreClientError> {
        let path = conversation_path(conversation_id);
        let request = UpdateConversationRequest {
            name: Some(name),
            name_source: "user".to_owned(),
        };
        let response: ApiResponse<ConversationResponse> = self
            .client
            .request_json(
                Method::PATCH,
                &path,
                Some(serde_json::to_value(request).map_err(|_| CoreClientError::Decode)?),
            )
            .await?;
        let conversation = response.data.ok_or(CoreClientError::Decode)?;
        self.refresh_list().await?;
        if self.snapshot().selected_conversation_id.as_deref() == Some(conversation_id) {
            self.load_conversation(conversation_id).await?;
        }
        Ok(conversation)
    }

    pub async fn load_conversation(&self, conversation_id: &str) -> Result<(), CoreClientError> {
        self.set_loading(true);
        let result = async {
            let detail_path = conversation_path(conversation_id);
            let messages_path = format!("{detail_path}/messages");
            let detail: ApiResponse<ConversationResponse> =
                self.client.request_json(Method::GET, &detail_path, None).await?;
            let messages: ApiResponse<MessageListResponse> =
                self.client.request_json(Method::GET, &messages_path, None).await?;
            let confirmations = self.client.list_confirmations(conversation_id).await?;
            let detail = detail.data.ok_or(CoreClientError::Decode)?;
            let messages = messages.data.ok_or(CoreClientError::Decode)?;
            let mut state = self.state.lock().expect("conversation state mutex poisoned");
            self.message_reducer
                .lock()
                .expect("message reducer mutex poisoned")
                .replace_history(&messages.items);
            state.selected_conversation_id = Some(conversation_id.to_owned());
            state.selected_conversation = Some(detail);
            state.messages = messages.items;
            state.confirmations = confirmations;
            state.error = None;
            Ok(())
        }
        .await;
        self.set_loading(false);
        result
    }

    pub async fn send_message(
        &self,
        conversation_id: &str,
        content: String,
        files: Vec<std::path::PathBuf>,
    ) -> Result<SendMessageResponse, CoreClientError> {
        let mut file_refs = Vec::new();
        for path in files {
            let file_name = path
                .file_name()
                .and_then(|name| name.to_str())
                .ok_or(CoreClientError::Decode)?;
            let uploaded_path = self.client.upload_file(&path, file_name, conversation_id).await?;
            file_refs.push(ChatFileRef::Upload { path: uploaded_path });
        }
        let request = SendMessageRequest {
            content: content.clone(),
            files: file_refs,
            inject_skills: Vec::new(),
            hidden: false,
        };
        let path = format!("/api/conversations/{conversation_id}/messages");
        let response: ApiResponse<SendMessageResponse> = self
            .client
            .request_json(
                Method::POST,
                &path,
                Some(serde_json::to_value(request).map_err(|_| CoreClientError::Decode)?),
            )
            .await?;
        let response = response.data.ok_or(CoreClientError::Decode)?;
        self.message_reducer
            .lock()
            .expect("message reducer mutex poisoned")
            .begin_turn(
                conversation_id,
                response.msg_id.clone(),
                response.turn_id.clone(),
                content,
                response.runtime.clone(),
            );
        Ok(response)
    }

    pub async fn respond_confirmation(
        &self,
        conversation_id: &str,
        confirmation: &Confirmation,
        data: Value,
        always_allow: bool,
    ) -> Result<(), CoreClientError> {
        self.client
            .confirm(
                conversation_id,
                &confirmation.call_id,
                ConfirmRequest {
                    msg_id: confirmation.id.clone(),
                    data,
                    always_allow,
                },
            )
            .await?;
        self.remove_confirmation(&confirmation.id);
        Ok(())
    }

    pub async fn answer_ask(
        &self,
        conversation_id: &str,
        request_id: &str,
        request: AskAnswerRequest,
        confirmation_id: &str,
    ) -> Result<(), CoreClientError> {
        self.client.answer_ask(conversation_id, request_id, request).await?;
        self.remove_confirmation(confirmation_id);
        Ok(())
    }

    pub async fn cancel_message(
        &self,
        conversation_id: &str,
        turn_id: &str,
    ) -> Result<CancelConversationResponse, CoreClientError> {
        let path = format!("/api/conversations/{conversation_id}/cancel");
        let request = CancelConversationRequest {
            turn_id: turn_id.to_owned(),
        };
        let response: ApiResponse<CancelConversationResponse> = self
            .client
            .request_json(
                Method::POST,
                &path,
                Some(serde_json::to_value(request).map_err(|_| CoreClientError::Decode)?),
            )
            .await?;
        let response = response.data.ok_or(CoreClientError::Decode)?;
        self.message_reducer
            .lock()
            .expect("message reducer mutex poisoned")
            .set_cancelling(response.runtime.clone());
        Ok(response)
    }

    pub fn apply_websocket_event(&self, name: &str, data: &Value) -> ConversationEventAction {
        match name {
            "message.stream" => {
                let selected = self.snapshot().selected_conversation_id;
                let is_selected = selected.as_deref() == data.get("conversation_id").and_then(Value::as_str);
                if is_selected {
                    self.message_reducer
                        .lock()
                        .expect("message reducer mutex poisoned")
                        .apply_stream(data);
                }
                ConversationEventAction::Ignored
            }
            "message.userCreated" => {
                let selected = self.snapshot().selected_conversation_id;
                let is_selected = selected.as_deref() == data.get("conversation_id").and_then(Value::as_str);
                if is_selected {
                    self.message_reducer
                        .lock()
                        .expect("message reducer mutex poisoned")
                        .apply_user_created(data);
                }
                ConversationEventAction::Ignored
            }
            "confirmation.remove" => {
                let is_selected = self.snapshot().selected_conversation_id.as_deref()
                    == data.get("conversation_id").and_then(Value::as_str);
                if is_selected {
                    if let Some(id) = data.get("id").and_then(Value::as_str) {
                        self.remove_confirmation(id);
                    } else {
                        return ConversationEventAction::RefreshSelected;
                    }
                }
                ConversationEventAction::Ignored
            }
            "conversation.nameUpdated" => {
                match serde_json::from_value::<ConversationNameUpdatedPayload>(data.clone()) {
                    Ok(payload) => {
                        let mut state = self.state.lock().expect("conversation state mutex poisoned");
                        for conversation in &mut state.conversations {
                            if conversation.id == payload.conversation_id {
                                conversation.name = payload.name.clone();
                            }
                        }
                        if let Some(conversation) = &mut state.selected_conversation
                            && conversation.id == payload.conversation_id
                        {
                            conversation.name = payload.name;
                        }
                        ConversationEventAction::RefreshList
                    }
                    Err(_) => ConversationEventAction::RefreshList,
                }
            }
            "conversation.listChanged" => {
                let Some(conversation_id) = data.get("conversation_id").and_then(Value::as_str) else {
                    return ConversationEventAction::RefreshList;
                };
                if conversation_id.is_empty() {
                    return ConversationEventAction::RefreshList;
                }
                if self.snapshot().selected_conversation_id.as_deref() == Some(conversation_id) {
                    ConversationEventAction::RefreshSelected
                } else {
                    ConversationEventAction::RefreshList
                }
            }
            _ => ConversationEventAction::Ignored,
        }
    }

    pub fn event_action_for_payload(name: &str, data: &Value) -> ConversationEventAction {
        if name == "conversation.listChanged" && data.get("conversation_id").and_then(Value::as_str).is_none() {
            return ConversationEventAction::RefreshList;
        }
        ConversationEventAction::Ignored
    }

    fn remove_confirmation(&self, confirmation_id: &str) {
        let mut state = self.state.lock().expect("conversation state mutex poisoned");
        state
            .confirmations
            .retain(|confirmation| confirmation.id != confirmation_id);
    }

    fn set_loading(&self, is_loading: bool) {
        self.state.lock().expect("conversation state mutex poisoned").is_loading = is_loading;
    }
}

fn conversation_path(conversation_id: &str) -> String {
    format!("/api/conversations/{conversation_id}")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ConversationEventAction, ConversationStore};

    #[test]
    fn malformed_conversation_event_requires_rest_refresh() {
        let action = ConversationStore::event_action_for_payload("conversation.listChanged", &json!({}));
        assert_eq!(action, ConversationEventAction::RefreshList);
    }
}
