//! Shared UI service types.

#[derive(Clone, Debug, Default, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    #[default]
    Active,
    Idle,
    Pending,
    InProgress,
    Completed,
    Failed,
    Closed,
}
