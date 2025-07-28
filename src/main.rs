//! Main entry point for the Automata workflow engine

use automata::{
    api::server::ApiServer,
    config::AppConfig,
    core::engine::ExecutionEngine,
    nodes::NodeRegistry,
    state::{PostgresStateManager, StateManager, StateManagerTrait},
};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = AppConfig::load().unwrap_or_else(|e| {
        eprintln!("Failed to load configuration: {e}. Using defaults.");
        AppConfig::default()
    });

    // Initialize logging
    let log_level = match config.logging.level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .with_thread_ids(true)
        .with_thread_names(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    info!("Starting Automata Workflow Engine v{}", automata::VERSION);
    info!("Configuration loaded from config/app.yml");

    // Initialize components
    let node_registry = Arc::new(NodeRegistry::new());

    // Initialize state manager based on configuration
    let state_manager: Arc<dyn StateManagerTrait> = if config.database.url == "memory" {
        info!("Using in-memory state manager");
        Arc::new(StateManager::new())
    } else {
        info!("Connecting to PostgreSQL database...");
        match PostgresStateManager::new(&config.database.url).await {
            Ok(postgres_manager) => {
                info!("Successfully connected to PostgreSQL");
                Arc::new(postgres_manager)
            }
            Err(e) => {
                eprintln!(
                    "Failed to connect to PostgreSQL: {e}. Falling back to in-memory storage."
                );
                Arc::new(StateManager::new())
            }
        }
    };

    let engine_config = config.to_execution_engine_config();

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

    info!(
        host = %config.server.host,
        port = config.server.port,
        "Starting API server..."
    );

    info!(
        "API documentation available at http://{}:{}/swagger-ui/",
        config.server.host, config.server.port
    );

    api_server.serve(config.server.port).await?;

    Ok(())
}
