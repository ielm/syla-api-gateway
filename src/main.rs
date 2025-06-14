use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod auth;
mod clients;
mod error;
mod execution;
mod grpc;
mod proto;
mod state;

use error::ApiError;
use state::AppState;

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    timestamp: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "syla_api_gateway=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize application state
    let state = Arc::new(AppState::new().await?);

    // Get configuration
    let rest_port = std::env::var("REST_PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("Invalid REST_PORT");
    
    let grpc_port = std::env::var("GRPC_PORT")
        .unwrap_or_else(|_| "8081".to_string())
        .parse::<u16>()
        .expect("Invalid GRPC_PORT");

    let auth_service_url = std::env::var("AUTH_SERVICE_URL")
        .unwrap_or_else(|_| "http://localhost:8085".to_string());
    
    let skip_auth = std::env::var("SKIP_AUTH")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    // Create auth interceptor
    let auth_interceptor = auth::AuthInterceptor::new(auth_service_url, skip_auth);

    // Create gRPC service
    let grpc_service = grpc::SylaGatewayService::new(state.clone(), auth_interceptor);
    let grpc_server = proto::SylaGatewayServer::new(grpc_service);

    // Build REST router
    let rest_app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/executions", post(create_execution))
        .route("/v1/executions/:id", get(get_execution))
        .route("/v1/executions/:id/status", get(get_execution_status))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10MB limit
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start both servers
    let rest_addr = SocketAddr::from(([0, 0, 0, 0], rest_port));
    let grpc_addr = SocketAddr::from(([0, 0, 0, 0], grpc_port));

    tracing::info!("Starting REST API on {}", rest_addr);
    tracing::info!("Starting gRPC API on {}", grpc_addr);

    // Spawn REST server
    let rest_handle = tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(rest_addr)
            .await
            .expect("Failed to bind REST listener");
        axum::serve(listener, rest_app)
            .await
            .expect("REST server failed");
    });

    // Spawn gRPC server
    let grpc_handle = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(grpc_server)
            .serve(grpc_addr)
            .await
            .expect("gRPC server failed");
    });

    // Wait for both servers
    tokio::try_join!(rest_handle, grpc_handle)?;

    Ok(())
}

async fn health_handler() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
    })
}

async fn create_execution(
    State(state): State<Arc<AppState>>,
    Json(request): Json<execution::CreateExecutionRequest>,
) -> Result<Json<execution::ExecutionResponse>, ApiError> {
    let execution = state.create_execution(request).await?;
    Ok(Json(execution))
}

async fn get_execution(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<execution::ExecutionResponse>, ApiError> {
    let execution = state.get_execution(id).await?;
    Ok(Json(execution))
}

async fn get_execution_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<execution::ExecutionStatus>, ApiError> {
    let status = state.get_execution_status(id).await?;
    Ok(Json(status))
}