use std::sync::{Arc, RwLock};

use reqwest::Method;

use super::client::{CoreClient, CoreClientError};
use super::models::{ApiResponse, ProjectDetailResponse, ProjectListItemResponse, WorkspaceFlatFileResponse};

#[derive(Clone, Debug, Default)]
pub struct ProjectState {
    pub projects: Vec<ProjectListItemResponse>,
    pub selected_project_id: Option<String>,
    pub project: Option<ProjectDetailResponse>,
    pub files: Vec<WorkspaceFlatFileResponse>,
    pub error: Option<String>,
    pub is_loading: bool,
}

#[derive(Clone)]
pub struct ProjectStore {
    client: Arc<CoreClient>,
    state: Arc<RwLock<ProjectState>>,
}

impl ProjectStore {
    pub fn new(client: Arc<CoreClient>) -> Self {
        Self {
            client,
            state: Arc::new(RwLock::new(ProjectState::default())),
        }
    }

    pub fn snapshot(&self) -> ProjectState {
        self.state.read().expect("project state lock poisoned").clone()
    }

    pub async fn refresh_list(&self) -> Result<Vec<ProjectListItemResponse>, CoreClientError> {
        self.set_loading(true);
        let response: Result<ApiResponse<Vec<ProjectListItemResponse>>, _> =
            self.client.request_json(Method::GET, "/api/projects", None).await;
        let projects = response.and_then(|response| response.data.ok_or(CoreClientError::Decode));
        let mut state = self.state.write().expect("project state lock poisoned");
        state.is_loading = false;
        match projects {
            Ok(projects) => {
                state.projects = projects.clone();
                state.error = None;
                Ok(projects)
            }
            Err(error) => {
                state.error = Some("Unable to load projects".to_owned());
                Err(error)
            }
        }
    }

    pub async fn select(&self, project_id: &str) -> Result<(), CoreClientError> {
        self.set_loading(true);
        let result = async {
            let project_path = format!("/api/projects/{project_id}");
            let response: ApiResponse<ProjectDetailResponse> =
                self.client.request_json(Method::GET, &project_path, None).await?;
            let project = response.data.ok_or(CoreClientError::Decode)?;
            let files = match workspace_root(&project) {
                Some(root) => {
                    let response: ApiResponse<Vec<WorkspaceFlatFileResponse>> = self
                        .client
                        .request_json(Method::POST, "/api/fs/list", Some(serde_json::json!({ "root": root })))
                        .await?;
                    response.data.ok_or(CoreClientError::Decode)?
                }
                None => Vec::new(),
            };
            Ok::<_, CoreClientError>((project, files))
        }
        .await;

        let mut state = self.state.write().expect("project state lock poisoned");
        state.is_loading = false;
        match result {
            Ok((project, files)) => {
                state.selected_project_id = Some(project_id.to_owned());
                state.project = Some(project);
                state.files = files;
                state.error = None;
                Ok(())
            }
            Err(error) => {
                state.error = Some("Unable to load project details".to_owned());
                Err(error)
            }
        }
    }

    fn set_loading(&self, loading: bool) {
        self.state.write().expect("project state lock poisoned").is_loading = loading;
    }
}

fn workspace_root(project: &ProjectDetailResponse) -> Option<&str> {
    project
        .explorer
        .entries
        .iter()
        .find(|entry| entry.pe_id == project.explorer.workspace_pe_id && entry.runtime_status == "available")
        .map(|entry| entry.display_path.as_str())
}
