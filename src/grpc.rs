use std::sync::Arc;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use crate::{
    auth::AuthInterceptor,
    proto::*,
    state::AppState,
};
use tracing::{debug, info, error};

/// gRPC service implementation for Syla Gateway
pub struct SylaGatewayService {
    state: Arc<AppState>,
    auth_interceptor: AuthInterceptor,
}

impl SylaGatewayService {
    pub fn new(state: Arc<AppState>, auth_interceptor: AuthInterceptor) -> Self {
        Self {
            state,
            auth_interceptor,
        }
    }
}

#[tonic::async_trait]
impl SylaGateway for SylaGatewayService {
    async fn create_execution(
        &self,
        request: Request<CreateExecutionRequest>,
    ) -> Result<Response<CreateExecutionResponse>, Status> {
        // Authenticate the request
        let auth_context = self.auth_interceptor.authenticate(&request).await?;
        debug!("Authenticated user: {}", auth_context.user_id);

        let req = request.into_inner();
        
        // Convert Language enum to string
        let language = match Language::try_from(req.language) {
            Ok(Language::Python) => "python",
            Ok(Language::Javascript) => "javascript",
            Ok(Language::Typescript) => "typescript",
            Ok(Language::Rust) => "rust",
            Ok(Language::Go) => "go",
            Ok(Language::Java) => "java",
            Ok(Language::Cpp) => "cpp",
            Ok(Language::Csharp) => "csharp",
            Ok(Language::Ruby) => "ruby",
            Ok(Language::Php) => "php",
            _ => return Err(Status::invalid_argument("Invalid language")),
        };

        // Create execution request for backend service
        let execution_req = crate::execution::CreateExecutionRequest {
            code: req.code.clone(),
            language: language.to_string(),
            timeout_seconds: req.timeout.map(|t| t.seconds as u64),
            args: Some(req.args.clone()),
            workspace_id: if req.workspace_id.is_empty() {
                None
            } else {
                Uuid::parse_str(&req.workspace_id).ok()
            },
        };

        // Forward to execution service
        match self.state.create_execution(execution_req).await {
            Ok(exec_response) => {
                // Convert response to gRPC format
                let execution = Execution {
                    id: exec_response.id.to_string(),
                    user_id: auth_context.user_id,
                    workspace_id: "".to_string(), // TODO: Handle workspace
                    status: match exec_response.status {
                        crate::execution::ExecutionStatus::Pending => ExecutionStatus::Pending as i32,
                        crate::execution::ExecutionStatus::Running => ExecutionStatus::Running as i32,
                        crate::execution::ExecutionStatus::Completed => ExecutionStatus::Completed as i32,
                        crate::execution::ExecutionStatus::Failed => ExecutionStatus::Failed as i32,
                        crate::execution::ExecutionStatus::Timeout => ExecutionStatus::Timeout as i32,
                    },
                    language: req.language,
                    code: req.code.clone(),
                    args: req.args.clone(),
                    result: exec_response.result.map(|r| ExecutionResult {
                        exit_code: r.exit_code,
                        stdout: r.stdout,
                        stderr: r.stderr,
                        execution_time: Some(prost_types::Duration {
                            seconds: (r.duration_ms / 1000) as i64,
                            nanos: ((r.duration_ms % 1000) * 1_000_000) as i32,
                        }),
                        files_created: vec![],
                        outputs: Default::default(),
                        error: None,
                    }),
                    resource_usage: None,
                    created_at: Some(prost_types::Timestamp {
                        seconds: exec_response.created_at.timestamp(),
                        nanos: exec_response.created_at.timestamp_subsec_nanos() as i32,
                    }),
                    started_at: exec_response.started_at.map(|t| prost_types::Timestamp {
                        seconds: t.timestamp(),
                        nanos: t.timestamp_subsec_nanos() as i32,
                    }),
                    completed_at: exec_response.completed_at.map(|t| prost_types::Timestamp {
                        seconds: t.timestamp(),
                        nanos: t.timestamp_subsec_nanos() as i32,
                    }),
                    metadata: req.metadata,
                };

                Ok(Response::new(CreateExecutionResponse {
                    execution: Some(execution),
                }))
            }
            Err(e) => {
                error!("Failed to create execution: {}", e);
                Err(Status::internal("Failed to create execution"))
            }
        }
    }

    async fn get_execution(
        &self,
        request: Request<GetExecutionRequest>,
    ) -> Result<Response<GetExecutionResponse>, Status> {
        // Authenticate the request
        let auth_context = self.auth_interceptor.authenticate(&request).await?;
        
        let req = request.into_inner();
        let execution_id = Uuid::parse_str(&req.id)
            .map_err(|_| Status::invalid_argument("Invalid execution ID"))?;

        match self.state.get_execution(execution_id).await {
            Ok(exec_response) => {
                // Convert response to gRPC format
                let execution = Execution {
                    id: exec_response.id.to_string(),
                    user_id: auth_context.user_id,
                    workspace_id: "".to_string(),
                    status: match exec_response.status {
                        crate::execution::ExecutionStatus::Pending => ExecutionStatus::Pending as i32,
                        crate::execution::ExecutionStatus::Running => ExecutionStatus::Running as i32,
                        crate::execution::ExecutionStatus::Completed => ExecutionStatus::Completed as i32,
                        crate::execution::ExecutionStatus::Failed => ExecutionStatus::Failed as i32,
                        crate::execution::ExecutionStatus::Timeout => ExecutionStatus::Timeout as i32,
                    },
                    language: Language::Unspecified as i32, // TODO: Store language
                    code: String::new(), // TODO: Store code
                    args: vec![],
                    result: exec_response.result.map(|r| ExecutionResult {
                        exit_code: r.exit_code,
                        stdout: r.stdout,
                        stderr: r.stderr,
                        execution_time: Some(prost_types::Duration {
                            seconds: (r.duration_ms / 1000) as i64,
                            nanos: ((r.duration_ms % 1000) * 1_000_000) as i32,
                        }),
                        files_created: vec![],
                        outputs: Default::default(),
                        error: None,
                    }),
                    resource_usage: None,
                    created_at: Some(prost_types::Timestamp {
                        seconds: exec_response.created_at.timestamp(),
                        nanos: exec_response.created_at.timestamp_subsec_nanos() as i32,
                    }),
                    started_at: exec_response.started_at.map(|t| prost_types::Timestamp {
                        seconds: t.timestamp(),
                        nanos: t.timestamp_subsec_nanos() as i32,
                    }),
                    completed_at: exec_response.completed_at.map(|t| prost_types::Timestamp {
                        seconds: t.timestamp(),
                        nanos: t.timestamp_subsec_nanos() as i32,
                    }),
                    metadata: Default::default(),
                };

                Ok(Response::new(GetExecutionResponse {
                    execution: Some(execution),
                }))
            }
            Err(e) => {
                if let crate::error::ApiError::NotFound = e {
                    Err(Status::not_found("Execution not found"))
                } else {
                    error!("Failed to get execution: {}", e);
                    Err(Status::internal("Failed to get execution"))
                }
            }
        }
    }

    async fn list_executions(
        &self,
        _request: Request<ListExecutionsRequest>,
    ) -> Result<Response<ListExecutionsResponse>, Status> {
        // TODO: Implement list executions
        Err(Status::unimplemented("List executions not yet implemented"))
    }

    async fn cancel_execution(
        &self,
        _request: Request<CancelExecutionRequest>,
    ) -> Result<Response<CancelExecutionResponse>, Status> {
        // TODO: Implement cancel execution
        Err(Status::unimplemented("Cancel execution not yet implemented"))
    }

    type StreamExecutionStream = tokio_stream::wrappers::ReceiverStream<Result<StreamExecutionResponse, Status>>;

    async fn stream_execution(
        &self,
        _request: Request<StreamExecutionRequest>,
    ) -> Result<Response<Self::StreamExecutionStream>, Status> {
        // TODO: Implement execution streaming
        Err(Status::unimplemented("Stream execution not yet implemented"))
    }

    async fn create_workspace(
        &self,
        _request: Request<CreateWorkspaceRequest>,
    ) -> Result<Response<CreateWorkspaceResponse>, Status> {
        // TODO: Implement workspace creation
        Err(Status::unimplemented("Create workspace not yet implemented"))
    }

    async fn get_workspace(
        &self,
        _request: Request<GetWorkspaceRequest>,
    ) -> Result<Response<GetWorkspaceResponse>, Status> {
        // TODO: Implement get workspace
        Err(Status::unimplemented("Get workspace not yet implemented"))
    }

    async fn list_workspaces(
        &self,
        _request: Request<ListWorkspacesRequest>,
    ) -> Result<Response<ListWorkspacesResponse>, Status> {
        // TODO: Implement list workspaces
        Err(Status::unimplemented("List workspaces not yet implemented"))
    }

    async fn update_workspace(
        &self,
        _request: Request<UpdateWorkspaceRequest>,
    ) -> Result<Response<UpdateWorkspaceResponse>, Status> {
        // TODO: Implement update workspace
        Err(Status::unimplemented("Update workspace not yet implemented"))
    }

    async fn delete_workspace(
        &self,
        _request: Request<DeleteWorkspaceRequest>,
    ) -> Result<Response<DeleteWorkspaceResponse>, Status> {
        // TODO: Implement delete workspace
        Err(Status::unimplemented("Delete workspace not yet implemented"))
    }

    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        info!("Health check request received");
        
        let mut components = std::collections::HashMap::new();
        
        // Check execution service health
        components.insert(
            "execution_service".to_string(),
            ComponentHealth {
                healthy: true, // TODO: Actually check service health
                message: "Service is running".to_string(),
                details: Default::default(),
            },
        );

        Ok(Response::new(HealthCheckResponse {
            status: health_check_response::HealthStatus::Healthy as i32,
            components,
            timestamp: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
        }))
    }

    async fn get_metrics(
        &self,
        _request: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        // TODO: Implement metrics collection
        Err(Status::unimplemented("Get metrics not yet implemented"))
    }
}