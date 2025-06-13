use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateExecutionRequest {
    pub code: String,
    pub language: String,
    pub timeout_seconds: Option<u64>,
    pub args: Option<Vec<String>>,
    pub workspace_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExecutionResponse {
    pub id: Uuid,
    pub status: ExecutionStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<ExecutionResult>,
}

#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Serialize, Clone)]
pub struct ExecutionResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

impl ExecutionResponse {
    pub fn new_pending() -> Self {
        Self {
            id: Uuid::new_v4(),
            status: ExecutionStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
        }
    }
}