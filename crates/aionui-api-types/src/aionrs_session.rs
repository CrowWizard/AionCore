use serde::{Deserialize, Serialize};

/// Read-only summary of a persisted aionrs session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AionrsSessionResponse {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub model: String,
    pub summary: String,
    pub message_count: usize,
}
