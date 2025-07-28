//! API server implementation using Axum

use crate::api::handlers::{
    create_workflow, execute_workflow, get_execution, get_workflow, list_nodes,
};
use crate::core::engine::ExecutionEngine;
use crate::nodes::NodeRegistry;
use crate::state::StateManager;
use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing::info;

/// API server for the workflow engine
pub struct ApiServer {
    execution_engine: Arc<ExecutionEngine>,
    node_registry: Arc<NodeRegistry>,
    state_manager: Arc<StateManager>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(
        execution_engine: Arc<ExecutionEngine>,
        node_registry: Arc<NodeRegistry>,
        state_manager: Arc<StateManager>,
    ) -> Self {
        Self {
            execution_engine,
            node_registry,
            state_manager,
        }
    }

    /// Create the application router
    fn create_router(&self) -> Router {
        let shared_state = crate::api::models::SharedState {
            execution_engine: self.execution_engine.clone(),
            node_registry: self.node_registry.clone(),
            state_manager: self.state_manager.clone(),
        };

        Router::new()
            .route("/health", get(health_check))
            .route("/api/v1/workflows", post(create_workflow))
            .route("/api/v1/workflows/:id", get(get_workflow))
            .route("/api/v1/workflows/:id/execute", post(execute_workflow))
            .route("/api/v1/executions/:id", get(get_execution))
            .route("/api/v1/nodes", get(list_nodes))
            .with_state(shared_state)
            .layer(CorsLayer::permissive())
    }

    /// Start the API server
    pub async fn serve(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let app = self.create_router();
        let addr = format!("0.0.0.0:{port}");

        info!(address = %addr, "API server starting");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

// Handler functions
async fn health_check() -> &'static str {
    "OK"
}
