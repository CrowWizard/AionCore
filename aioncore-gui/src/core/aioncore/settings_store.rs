use std::sync::{Arc, RwLock};

use reqwest::Method;
use serde_json::{Value, json};

use super::client::{CoreClient, CoreClientError};
use super::models::{ApiResponse, AssistantResponse, SkillListItemResponse, SystemSettingsResponse};

#[derive(Clone, Debug, Default)]
pub struct SettingsState {
    pub settings: Option<SystemSettingsResponse>,
    pub skills: Vec<SkillListItemResponse>,
    pub assistants: Vec<AssistantResponse>,
    pub is_loading: bool,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct SettingsStore {
    client: Arc<CoreClient>,
    state: Arc<RwLock<SettingsState>>,
}

impl SettingsStore {
    pub fn new(client: Arc<CoreClient>) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(SettingsState::default())),
        }
    }

    pub fn snapshot(&self) -> SettingsState {
        self.state.read().expect("settings state lock poisoned").clone()
    }

    pub async fn refresh(&self) -> Result<(), CoreClientError> {
        self.set_loading(true);
        let result = async {
            let settings: ApiResponse<SystemSettingsResponse> =
                self.client.request_json(Method::GET, "/api/settings", None).await?;
            let skills: ApiResponse<Vec<SkillListItemResponse>> =
                self.client.request_json(Method::GET, "/api/skills", None).await?;
            let assistants: ApiResponse<Vec<AssistantResponse>> =
                self.client.request_json(Method::GET, "/api/assistants", None).await?;
            Ok::<_, CoreClientError>((
                settings.data.ok_or(CoreClientError::Decode)?,
                skills.data.ok_or(CoreClientError::Decode)?,
                assistants.data.ok_or(CoreClientError::Decode)?,
            ))
        }
        .await;

        let mut state = self.state.write().expect("settings state lock poisoned");
        state.is_loading = false;
        match result {
            Ok((settings, skills, assistants)) => {
                state.settings = Some(settings);
                state.skills = skills;
                state.assistants = assistants;
                state.error = None;
                Ok(())
            }
            Err(error) => {
                state.error = Some("Unable to load AionCore settings".to_owned());
                Err(error)
            }
        }
    }

    pub async fn update_setting(&self, field: &str, value: Value) -> Result<SystemSettingsResponse, CoreClientError> {
        let mut body = serde_json::Map::new();
        body.insert(field.to_owned(), value);
        let response: ApiResponse<SystemSettingsResponse> = self
            .client
            .request_json(Method::PATCH, "/api/settings", Some(Value::Object(body)))
            .await?;
        let settings = response.data.ok_or(CoreClientError::Decode)?;
        let mut state = self.state.write().expect("settings state lock poisoned");
        state.settings = Some(settings.clone());
        state.error = None;
        Ok(settings)
    }

    pub async fn set_notification_enabled(&self, enabled: bool) -> Result<SystemSettingsResponse, CoreClientError> {
        self.update_setting("notification_enabled", json!(enabled)).await
    }

    fn set_loading(&self, is_loading: bool) {
        self.state.write().expect("settings state lock poisoned").is_loading = is_loading;
    }
}
