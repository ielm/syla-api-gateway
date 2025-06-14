use crate::clients::execution::ExecutionClient;
use crate::error::ApiError;
use crate::execution::{CreateExecutionRequest, ExecutionResponse, ExecutionStatus};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub struct AppState {
    execution_client: Arc<RwLock<ExecutionClient>>,
    // In-memory cache for MVP (will be Redis later)
    executions: Arc<RwLock<HashMap<Uuid, ExecutionResponse>>>,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        let execution_service_url = std::env::var("EXECUTION_SERVICE_URL")
            .unwrap_or_else(|_| "http://localhost:8081".to_string());

        let execution_client = ExecutionClient::new(&execution_service_url).await?;

        Ok(Self {
            execution_client: Arc::new(RwLock::new(execution_client)),
            executions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub async fn create_execution(
        &self,
        request: CreateExecutionRequest,
    ) -> Result<ExecutionResponse, ApiError> {
        // TODO: Get user_id from auth context
        let user_id = "test-user".to_string();
        let workspace_id = request.workspace_id.map(|id| id.to_string());
        
        // Send to execution service via gRPC
        let mut client = self.execution_client.write().await;
        let execution = client.create_execution(user_id, workspace_id, request).await?;
        
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
        
        // Fetch from execution service via gRPC
        let mut client = self.execution_client.write().await;
        let execution = client.get_execution(id).await?;
        
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