//! Main entry point for the Automata workflow engine

use automata::{
    api::server::ApiServer,
    core::engine::{ExecutionEngine, ExecutionEngineConfig},
    nodes::NodeRegistry,
    state::StateManager,
    utils::config,
};
use std::sync::Arc;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Automata Workflow Engine v{}", automata::VERSION);

    // Load configuration
    let api_port = config::env_var_or_default_int("API_PORT", 8080);
    let worker_threads = config::env_var_or_default_int("WORKER_THREADS", 4);

    info!(
        api_port = api_port,
        worker_threads = worker_threads,
        "Configuration loaded"
    );

    // Initialize components
    let node_registry = Arc::new(NodeRegistry::new());
    let state_manager = Arc::new(StateManager::new());

    let engine_config = ExecutionEngineConfig {
        max_concurrent_executions: 1000,
        max_concurrent_nodes: 100,
        default_node_timeout: std::time::Duration::from_secs(30),
        max_execution_timeout: std::time::Duration::from_secs(300),
        enable_checkpointing: true,
        checkpoint_interval: std::time::Duration::from_secs(30),
    };

    let execution_engine = Arc::new(ExecutionEngine::new(
        node_registry.clone(),
        state_manager.clone(),
        engine_config,
    ));

    // Validate all nodes
    info!("Validating registered nodes...");
    let validation_results = node_registry.validate_all().await;
    let mut valid_count = 0;
    let mut error_count = 0;

    for result in validation_results {
        if result.is_valid {
            valid_count += 1;
        } else {
            error_count += 1;
            tracing::warn!(
                node_type = %result.node_type,
                errors = ?result.errors,
                "Node validation failed"
            );
        }
    }

    info!(
        valid_nodes = valid_count,
        error_nodes = error_count,
        "Node validation completed"
    );

    if error_count > 0 {
        tracing::warn!("Some nodes failed validation, but continuing...");
    }

    // Create and start API server
    let api_server = ApiServer::new(execution_engine, node_registry, state_manager);

    info!(port = api_port, "Starting API server...");
    api_server.serve(api_port as u16).await?;

    Ok(())
}
