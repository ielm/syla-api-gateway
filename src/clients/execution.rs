use crate::execution::{CreateExecutionRequest, ExecutionResponse, ExecutionResult, ExecutionStatus};
use crate::error::ApiError;
use anyhow::Result;
use tonic::{Request, Status};
use uuid::Uuid;

// Import the generated proto types
use crate::proto::execution::v1::{
    execution_service_client::ExecutionServiceClient,
    SubmitExecutionRequest, GetExecutionRequest, ExecutionRequest,
    Language, ExecutionMode, ExecutionStatus as ProtoExecutionStatus,
};
use crate::proto::common::v1::ExecutionContext;

pub struct ExecutionClient {
    client: ExecutionServiceClient<tonic::transport::Channel>,
}

impl ExecutionClient {
    pub async fn new(url: &str) -> Result<Self> {
        let channel = super::create_channel(url).await?;
        let client = ExecutionServiceClient::new(channel);
        Ok(Self { client })
    }
    
    pub async fn create_execution(
        &mut self,
        user_id: String,
        workspace_id: Option<String>,
        request: CreateExecutionRequest,
    ) -> Result<ExecutionResponse, ApiError> {
        let proto_request = SubmitExecutionRequest {
            context: Some(ExecutionContext {
                user_id,
                workspace_id: workspace_id.unwrap_or_default(),
                request_id: Uuid::new_v4().to_string(),
                session_id: String::new(),
                metadata: std::collections::HashMap::new(),
            }),
            request: Some(ExecutionRequest {
                code: request.code,
                language: self.language_to_proto(&request.language) as i32,
                args: request.args.unwrap_or_default(),
                environment: std::collections::HashMap::new(),
                resources: None,
                timeout: request.timeout_seconds.map(|s| prost_types::Duration {
                    seconds: s as i64,
                    nanos: 0,
                }),
                files: vec![],
                mode: ExecutionMode::Sandbox as i32,
                metadata: std::collections::HashMap::new(),
            }),
            r#async: true,
        };
        
        let response = self.client
            .submit_execution(Request::new(proto_request))
            .await
            .map_err(|e| ApiError::Internal(e.into()))?
            .into_inner();
        
        // Convert to ExecutionResponse
        Ok(ExecutionResponse {
            id: Uuid::parse_str(&response.execution_id)
                .map_err(|e| ApiError::Internal(e.into()))?,
            status: self.proto_to_status(response.status),
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            result: response.result.map(|r| ExecutionResult {
                exit_code: r.exit_code,
                stdout: r.stdout,
                stderr: r.stderr,
                duration_ms: 0, // TODO: Calculate from timestamps
            }),
        })
    }
    
    pub async fn get_execution(&mut self, id: Uuid) -> Result<ExecutionResponse, ApiError> {
        let request = GetExecutionRequest {
            execution_id: id.to_string(),
            include_output: true,
            include_metrics: false,
        };
        
        let response = self.client
            .get_execution(Request::new(request))
            .await
            .map_err(|e| match e.code() {
                tonic::Code::NotFound => ApiError::NotFound,
                _ => ApiError::Internal(e.into()),
            })?
            .into_inner();
        
        let execution = response.execution
            .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("Missing execution data")))?;
        
        // Convert to ExecutionResponse
        Ok(ExecutionResponse {
            id: Uuid::parse_str(&execution.id)
                .map_err(|e| ApiError::Internal(e.into()))?,
            status: self.proto_to_status(execution.status),
            created_at: execution.created_at
                .map(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
                .flatten()
                .unwrap_or_else(chrono::Utc::now),
            started_at: execution.started_at
                .map(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
                .flatten(),
            completed_at: execution.completed_at
                .map(|t| chrono::DateTime::from_timestamp(t.seconds, t.nanos as u32))
                .flatten(),
            result: execution.result.map(|r| ExecutionResult {
                exit_code: r.exit_code,
                stdout: r.stdout,
                stderr: r.stderr,
                duration_ms: 0, // TODO: Calculate from timestamps
            }),
        })
    }
    
    fn language_to_proto(&self, lang: &str) -> Language {
        match lang.to_lowercase().as_str() {
            "python" => Language::Python,
            "javascript" => Language::Javascript,
            "typescript" => Language::Typescript,
            "rust" => Language::Rust,
            "go" => Language::Go,
            "java" => Language::Java,
            "cpp" | "c++" => Language::Cpp,
            "csharp" | "c#" => Language::Csharp,
            "ruby" => Language::Ruby,
            "php" => Language::Php,
            "shell" | "bash" | "sh" => Language::Shell,
            _ => Language::Unspecified,
        }
    }
    
    fn proto_to_status(&self, status: i32) -> ExecutionStatus {
        match ProtoExecutionStatus::try_from(status).unwrap_or(ProtoExecutionStatus::Unspecified) {
            ProtoExecutionStatus::Pending | ProtoExecutionStatus::Queued | ProtoExecutionStatus::Preparing => ExecutionStatus::Pending,
            ProtoExecutionStatus::Running => ExecutionStatus::Running,
            ProtoExecutionStatus::Completed => ExecutionStatus::Completed,
            ProtoExecutionStatus::Failed | ProtoExecutionStatus::Cancelled => ExecutionStatus::Failed,
            ProtoExecutionStatus::Timeout => ExecutionStatus::Timeout,
            _ => ExecutionStatus::Pending,
        }
    }
}