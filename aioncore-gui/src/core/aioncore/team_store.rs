use std::sync::{Arc, RwLock};

use reqwest::Method;
use serde_json::Value;

use super::client::{CoreClient, CoreClientError};
use super::models::{
    ApiResponse, TeamActivityItemResponse, TeamActivityPageResponse, TeamMailboxMessageResponse, TeamTaskResponse,
};

#[derive(Clone, Debug, Default)]
pub struct TeamState {
    pub teams: Vec<TeamResponse>,
    pub selected_team_id: Option<String>,
    pub selected_team: Option<TeamResponse>,
    pub run_state: Option<TeamRunStateResponse>,
    pub activity: Vec<TeamActivityItemResponse>,
    pub mailbox: Vec<TeamMailboxMessageResponse>,
    pub tasks: Vec<TeamTaskResponse>,
    pub error: Option<String>,
    pub is_loading: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeamEventAction {
    RefreshList,
    RefreshSelected,
    Ignored,
}

#[derive(Clone)]
pub struct TeamStore {
    client: Arc<CoreClient>,
    state: Arc<RwLock<TeamState>>,
}

impl TeamStore {
    pub fn new(client: Arc<CoreClient>) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(TeamState::default())),
        }
    }

    pub fn snapshot(&self) -> TeamState {
        self.state.read().expect("team state lock poisoned").clone()
    }

    pub async fn refresh_list(&self) -> Result<Vec<TeamResponse>, CoreClientError> {
        self.set_loading(true);
        let response: Result<ApiResponse<Vec<TeamResponse>>, _> =
            self.client.request_json(Method::GET, "/api/teams", None).await;
        let teams = response.and_then(|response| response.data.ok_or(CoreClientError::Decode));
        let mut state = self.state.write().expect("team state lock poisoned");
        state.is_loading = false;
        match teams {
            Ok(teams) => {
                state.teams = teams.clone();
                state.error = None;
                Ok(teams)
            }
            Err(error) => {
                state.error = Some("Unable to load teams".to_owned());
                Err(error)
            }
        }
    }

    pub async fn select(&self, team_id: &str) -> Result<(), CoreClientError> {
        let team_path = format!("/api/teams/{team_id}");
        let run_state_path = format!("/api/teams/{team_id}/run-state");
        let activity_path = format!("/api/teams/{team_id}/activity?limit=50");
        let mailbox_path = format!("/api/teams/{team_id}/mailbox?limit=50");
        let tasks_path = format!("/api/teams/{team_id}/tasks?limit=50");
        let team: ApiResponse<TeamResponse> = self.client.request_json(Method::GET, &team_path, None).await?;
        let run_state: ApiResponse<TeamRunStateResponse> =
            self.client.request_json(Method::GET, &run_state_path, None).await?;
        let activity: ApiResponse<TeamActivityPageResponse> =
            self.client.request_json(Method::GET, &activity_path, None).await?;
        let mailbox: ApiResponse<Vec<TeamMailboxMessageResponse>> =
            self.client.request_json(Method::GET, &mailbox_path, None).await?;
        let tasks: ApiResponse<Vec<TeamTaskResponse>> =
            self.client.request_json(Method::GET, &tasks_path, None).await?;
        let mut state = self.state.write().expect("team state lock poisoned");
        state.selected_team_id = Some(team_id.to_owned());
        state.selected_team = Some(team.data.ok_or(CoreClientError::Decode)?);
        state.run_state = Some(run_state.data.ok_or(CoreClientError::Decode)?);
        state.activity = activity.data.ok_or(CoreClientError::Decode)?.items;
        state.mailbox = mailbox.data.ok_or(CoreClientError::Decode)?;
        state.tasks = tasks.data.ok_or(CoreClientError::Decode)?;
        state.error = None;
        Ok(())
    }

    pub fn apply_websocket_event(&self, name: &str, data: &Value) -> TeamEventAction {
        match name {
            "team.agentStatusChanged" => self.apply_agent_status(data),
            "team.agentSpawned" | "team.agentRemoved" | "team.agentRenamed" => TeamEventAction::RefreshSelected,
            "team.agentRuntimeStatusChanged" | "team.sessionStatusChanged" => self.apply_runtime_event(data),
            "team.taskChanged" => self.apply_task_change(data),
            "team.mailboxChanged" => self.apply_mailbox_change(data),
            "team.teammateMessage" => TeamEventAction::RefreshSelected,
            _ => TeamEventAction::Ignored,
        }
    }

    fn apply_agent_status(&self, data: &Value) -> TeamEventAction {
        let Ok(payload) = serde_json::from_value::<TeamAgentStatusPayload>(data.clone()) else {
            return TeamEventAction::RefreshList;
        };
        let mut state = self.state.write().expect("team state lock poisoned");
        if state.selected_team_id.as_deref() != Some(&payload.team_id) {
            return TeamEventAction::Ignored;
        }
        let Some(team) = state.selected_team.as_mut() else {
            return TeamEventAction::RefreshSelected;
        };
        let Some(agent) = team
            .assistants
            .iter_mut()
            .find(|agent| agent.slot_id == payload.slot_id)
        else {
            return TeamEventAction::RefreshSelected;
        };
        agent.status = Some(payload.status);
        TeamEventAction::Ignored
    }

    fn apply_runtime_event(&self, data: &Value) -> TeamEventAction {
        let team_id = serde_json::from_value::<TeamSessionStatusPayload>(data.clone())
            .map(|payload| payload.team_id)
            .or_else(|_| {
                serde_json::from_value::<TeamAgentRuntimeStatusPayload>(data.clone()).map(|payload| payload.team_id)
            });
        match team_id {
            Ok(team_id) if self.snapshot().selected_team_id.as_deref() == Some(&team_id) => {
                TeamEventAction::RefreshSelected
            }
            Ok(_) => TeamEventAction::Ignored,
            Err(_) => TeamEventAction::RefreshList,
        }
    }

    fn apply_task_change(&self, data: &Value) -> TeamEventAction {
        match serde_json::from_value::<TeamTaskChangedPayload>(data.clone()) {
            Ok(payload) if self.snapshot().selected_team_id.as_deref() == Some(&payload.team_id) => {
                TeamEventAction::RefreshSelected
            }
            Ok(_) => TeamEventAction::Ignored,
            Err(_) => TeamEventAction::RefreshList,
        }
    }

    fn apply_mailbox_change(&self, data: &Value) -> TeamEventAction {
        match serde_json::from_value::<TeamMailboxChangedPayload>(data.clone()) {
            Ok(payload) if self.snapshot().selected_team_id.as_deref() == Some(&payload.team_id) => {
                TeamEventAction::RefreshSelected
            }
            Ok(_) => TeamEventAction::Ignored,
            Err(_) => TeamEventAction::RefreshList,
        }
    }

    fn set_loading(&self, is_loading: bool) {
        self.state.write().expect("team state lock poisoned").is_loading = is_loading;
    }
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct TeamAgentRuntimeStatusPayload {
    pub team_id: String,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct TeamAgentStatusPayload {
    pub team_id: String,
    pub slot_id: String,
    pub status: String,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct TeamSessionStatusPayload {
    pub team_id: String,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct TeamTaskChangedPayload {
    pub team_id: String,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct TeamMailboxChangedPayload {
    pub team_id: String,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct TeamAgentResponse {
    pub slot_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct TeamResponse {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub workspace: String,
    #[serde(default, alias = "agents")]
    pub assistants: Vec<TeamAgentResponse>,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub struct TeamRunStateResponse {
    #[serde(default)]
    pub active_run: Option<Value>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{TeamAgentResponse, TeamEventAction, TeamResponse, TeamStore};
    use crate::core::aioncore::CoreClient;

    #[test]
    fn applies_selected_agent_status_without_refresh() {
        let store = TeamStore::new(std::sync::Arc::new(CoreClient::new("http://127.0.0.1:25808").unwrap()));
        {
            let mut state = store.state.write().unwrap();
            state.selected_team_id = Some("team-1".to_owned());
            state.selected_team = Some(TeamResponse {
                id: "team-1".to_owned(),
                name: "Team".to_owned(),
                workspace: String::new(),
                assistants: vec![TeamAgentResponse {
                    slot_id: "slot-1".to_owned(),
                    name: "Agent".to_owned(),
                    role: "lead".to_owned(),
                    status: None,
                }],
            });
        }
        assert_eq!(
            store.apply_websocket_event(
                "team.agentStatusChanged",
                &json!({"team_id":"team-1","slot_id":"slot-1","status":"working"}),
            ),
            TeamEventAction::Ignored
        );
        assert_eq!(
            store.snapshot().selected_team.unwrap().assistants[0].status.as_deref(),
            Some("working")
        );
    }

    #[test]
    fn requests_refresh_for_malformed_team_event() {
        let store = TeamStore::new(std::sync::Arc::new(CoreClient::new("http://127.0.0.1:25808").unwrap()));
        assert_eq!(
            store.apply_websocket_event("team.taskChanged", &json!({})),
            TeamEventAction::RefreshList
        );
    }

    #[test]
    fn ignores_activity_event_for_unselected_team() {
        let store = TeamStore::new(std::sync::Arc::new(CoreClient::new("http://127.0.0.1:25808").unwrap()));
        {
            let mut state = store.state.write().unwrap();
            state.selected_team_id = Some("team-1".to_owned());
        }
        assert_eq!(
            store.apply_websocket_event(
                "team.mailboxChanged",
                &json!({"team_id":"team-2","message":{},"change":"created"}),
            ),
            TeamEventAction::Ignored
        );
    }

    #[test]
    fn refreshes_selected_team_for_activity_event() {
        let store = TeamStore::new(std::sync::Arc::new(CoreClient::new("http://127.0.0.1:25808").unwrap()));
        {
            let mut state = store.state.write().unwrap();
            state.selected_team_id = Some("team-1".to_owned());
        }
        assert_eq!(
            store.apply_websocket_event(
                "team.taskChanged",
                &json!({"team_id":"team-1","task":{},"change":"updated"}),
            ),
            TeamEventAction::RefreshSelected
        );
    }
}
