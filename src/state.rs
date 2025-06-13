use crate::error::ApiError;
use crate::execution::{CreateExecutionRequest, ExecutionResponse, ExecutionResult, ExecutionStatus};
use anyhow::Result;
use reqwest::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

// Mirror types from execution service for deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionJob {
    id: Uuid,
    status: JobStatus,
    request: CreateExecutionRequest,
    created_at: chrono::DateTime<chrono::Utc>,
    started_at: Option<chrono::DateTime<chrono::Utc>>,
    completed_at: Option<chrono::DateTime<chrono::Utc>>,
    result: Option<JobResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
    duration_ms: u64,
}

pub struct AppState {
    client: Client,
    execution_service_url: String,
    // In-memory cache for MVP (will be Redis later)
    executions: Arc<RwLock<HashMap<Uuid, ExecutionResponse>>>,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let execution_service_url = std::env::var("EXECUTION_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8082".to_string());

        Ok(Self {
            client: Client::new(),
            execution_service_url,
            executions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn create_execution(
        &self,
        request: CreateExecutionRequest,
    ) -> Result<ExecutionResponse, ApiError> {
        // Send to execution service
        let response = self.client
            .post(format!("{}/executions", self.execution_service_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
            
        if !response.status().is_success() {
            return Err(ApiError::Internal(
                anyhow::anyhow!("Execution service returned status: {}", response.status())
            ));
        }
        
        let job: ExecutionJob = response.json().await
            .map_err(|e| ApiError::Internal(e.into()))?;
            
        // Convert job to ExecutionResponse
        let execution = ExecutionResponse {
            id: job.id,
            status: match job.status {
                JobStatus::Queued => ExecutionStatus::Pending,
                JobStatus::Running => ExecutionStatus::Running,
                JobStatus::Completed => ExecutionStatus::Completed,
                JobStatus::Failed => ExecutionStatus::Failed,
                JobStatus::Timeout => ExecutionStatus::Failed,
            },
            result: job.result.map(|r| ExecutionResult {
                exit_code: r.exit_code,
                stdout: r.stdout,
                stderr: r.stderr,
                duration_ms: r.duration_ms,
            }),
            created_at: job.created_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
        };
        
        // Cache the response
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution.id, execution.clone());
        }
        
        Ok(execution)
    }

    pub async fn get_execution(&self, id: Uuid) -> Result<ExecutionResponse, ApiError> {
        // Try cache first
        {
            let executions = self.executions.read().await;
            if let Some(execution) = executions.get(&id) {
                // If it's still pending/running, fetch latest from service
                if execution.status == ExecutionStatus::Pending || execution.status == ExecutionStatus::Running {
                    // Continue to fetch from service
                } else {
                    return Ok(execution.clone());
                }
            }
        }
        
        // Fetch from execution service
        let response = self.client
            .get(format!("{}/executions/{}", self.execution_service_url, id))
            .send()
            .await
            .map_err(|e| ApiError::Internal(e.into()))?;
            
        if response.status() == 404 {
            return Err(ApiError::NotFound);
        }
        
        if !response.status().is_success() {
            return Err(ApiError::Internal(
                anyhow::anyhow!("Execution service returned status: {}", response.status())
            ));
        }
        
        let job: ExecutionJob = response.json().await
            .map_err(|e| ApiError::Internal(e.into()))?;
            
        // Convert job to ExecutionResponse
        let execution = ExecutionResponse {
            id: job.id,
            status: match job.status {
                JobStatus::Queued => ExecutionStatus::Pending,
                JobStatus::Running => ExecutionStatus::Running,
                JobStatus::Completed => ExecutionStatus::Completed,
                JobStatus::Failed => ExecutionStatus::Failed,
                JobStatus::Timeout => ExecutionStatus::Failed,
            },
            result: job.result.map(|r| ExecutionResult {
                exit_code: r.exit_code,
                stdout: r.stdout,
                stderr: r.stderr,
                duration_ms: r.duration_ms,
            }),
            created_at: job.created_at,
            started_at: job.started_at,
            completed_at: job.completed_at,
        };
        
        // Update cache
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution.id, execution.clone());
        }
        
        Ok(execution)
    }

    pub async fn get_execution_status(&self, id: Uuid) -> Result<ExecutionStatus, ApiError> {
        let execution = self.get_execution(id).await?;
        Ok(execution.status)
    }
}