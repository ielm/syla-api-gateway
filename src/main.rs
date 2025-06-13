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

mod error;
mod execution;
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

    // Build router
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/v1/executions", post(create_execution))
        .route("/v1/executions/:id", get(get_execution))
        .route("/v1/executions/:id/status", get(get_execution_status))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024)) // 10MB limit
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Start server
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("Invalid PORT");
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting API gateway on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

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