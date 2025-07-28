//! Complete workflow demonstration showcasing all major features

use automata::{
    api::server::ApiServer,
    core::engine::{ExecutionEngine, ExecutionEngineConfig},
    dsl::DslParser,
    nodes::NodeRegistry,
    state::StateManager,
};
use serde_json::json;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("🚀 Starting Automata Workflow Engine Demo");

    // Initialize components
    let node_registry = Arc::new(NodeRegistry::new());
    let state_manager = Arc::new(StateManager::new());

    let engine_config = ExecutionEngineConfig {
        max_concurrent_executions: 1000,
        max_concurrent_nodes: 100,
        default_node_timeout: Duration::from_secs(30),
        max_execution_timeout: Duration::from_secs(300),
        enable_checkpointing: true,
        checkpoint_interval: Duration::from_secs(30),
    };

    let execution_engine = Arc::new(ExecutionEngine::new(
        node_registry.clone(),
        state_manager.clone(),
        engine_config,
    ));

    // Demo 1: Parse and execute a simple workflow
    demo_simple_workflow(&execution_engine, &state_manager).await?;

    // Demo 2: Complex workflow with multiple nodes and conditions
    demo_complex_workflow(&execution_engine, &state_manager).await?;

    // Demo 3: Node registry operations
    demo_node_registry(&node_registry).await?;

    // Demo 4: Start API server (optional - uncomment to run)
    // demo_api_server(execution_engine, node_registry, state_manager).await?;

    info!("✅ Demo completed successfully!");
    Ok(())
}

/// Demo 1: Simple workflow execution
async fn demo_simple_workflow(
    execution_engine: &Arc<ExecutionEngine>,
    state_manager: &Arc<StateManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n📋 Demo 1: Simple Workflow Execution");

    let yaml_workflow = r#"metadata:
  name: "User Registration Workflow"
  version: "1.0.0"
  description: "Process new user registration"

triggers:
  - webhook:
      path: "/api/register"
      method: "POST"

nodes:
  validate_input:
    type: validator
    rules:
      - field: email
        type: string
        required: true
      - field: name
        type: string
        required: true

  transform_data:
    type: transformer
    mapping:
      user_email: $validate_input.data.email
      user_name: $validate_input.data.name
      timestamp: $now()
      user_id: $uuid()

connections:
  - from: trigger
    to: validate_input
  - from: validate_input
    to: transform_data
    condition: $validate_input.success
"#;

    // Parse workflow
    let parser = DslParser::new();
    let parsed_workflow = parser.parse(yaml_workflow)?;

    info!(
        "✅ Parsed workflow: {} with {} nodes",
        parsed_workflow.definition.metadata.name,
        parsed_workflow.definition.nodes.len()
    );

    // Save workflow
    state_manager
        .save_workflow(&parsed_workflow.definition)
        .await?;
    info!(
        "✅ Workflow saved with ID: {}",
        parsed_workflow.definition.id
    );

    // Execute workflow
    let trigger_data = json!({
        "email": "john.doe@example.com",
        "name": "John Doe"
    });

    let result = execution_engine
        .execute_workflow(parsed_workflow.definition.id, trigger_data)
        .await?;

    info!(
        "✅ Workflow executed successfully! Execution ID: {}, Duration: {}ms",
        result.execution_id,
        result.duration_ms.unwrap_or(0)
    );

    Ok(())
}

/// Demo 2: Complex workflow with conditions and multiple paths
async fn demo_complex_workflow(
    execution_engine: &Arc<ExecutionEngine>,
    state_manager: &Arc<StateManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n📋 Demo 2: Complex Conditional Workflow");

    let yaml_workflow = r#"metadata:
  name: "Order Processing Workflow"
  version: "2.0.0"
  description: "Process customer orders with validation and routing"

triggers:
  - webhook:
      path: "/api/orders"
      method: "POST"

nodes:
  validate_order:
    type: validator
    rules:
      - field: amount
        type: number
        required: true
        minimum: 0.01
      - field: customer_id
        type: string
        required: true

  check_customer_status:
    type: transformer
    mapping:
      customer_id: $validate_order.data.customer_id
      amount: $validate_order.data.amount
      is_premium: true  # Simulated premium check
      credit_limit: 1000

  process_premium_order:
    type: transformer
    mapping:
      order_id: $uuid()
      customer_id: $check_customer_status.data.customer_id
      amount: $check_customer_status.data.amount
      status: "approved"
      processed_at: $now()
    condition: $check_customer_status.data.is_premium

  process_standard_order:
    type: transformer
    mapping:
      order_id: $uuid()
      customer_id: $check_customer_status.data.customer_id
      amount: $check_customer_status.data.amount
      status: "pending_review"
      processed_at: $now()
    condition: $not($check_customer_status.data.is_premium)

connections:
  - from: trigger
    to: validate_order
  - from: validate_order
    to: check_customer_status
    condition: $validate_order.success
  - from: check_customer_status
    to: process_premium_order
  - from: check_customer_status
    to: process_standard_order

test:
  - name: "Premium customer order"
    input:
      amount: 500.00
      customer_id: "cust_123"
    expect:
      process_premium_order:
        success: true
"#;

    // Parse and execute
    let parser = DslParser::new();
    let parsed_workflow = parser.parse(yaml_workflow)?;

    info!(
        "✅ Parsed complex workflow: {} with {} nodes and {} connections",
        parsed_workflow.definition.metadata.name,
        parsed_workflow.definition.nodes.len(),
        parsed_workflow.definition.connections.len()
    );

    // Save workflow
    state_manager
        .save_workflow(&parsed_workflow.definition)
        .await?;

    // Execute with premium customer
    let trigger_data = json!({
        "amount": 750.00,
        "customer_id": "premium_customer_456"
    });

    let result = execution_engine
        .execute_workflow(parsed_workflow.definition.id, trigger_data)
        .await?;

    info!(
        "✅ Complex workflow executed! ID: {}, Duration: {}ms",
        result.execution_id,
        result.duration_ms.unwrap_or(0)
    );

    // Show outputs if available
    if !result.outputs.is_empty() {
        let outputs = &result.outputs;
        info!("📤 Workflow outputs:");
        for (node_id, output) in outputs {
            info!("  {}: {}", node_id, output);
        }
    }

    Ok(())
}

/// Demo 3: Node registry operations
async fn demo_node_registry(
    node_registry: &Arc<NodeRegistry>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n📋 Demo 3: Node Registry Operations");

    // List all registered nodes
    let nodes = node_registry.list_nodes().await;
    info!("📦 Registered nodes ({}):", nodes.len());
    for node_type in nodes {
        info!("  - {}", node_type);
    }

    // Get node descriptions
    let descriptions = node_registry.get_all_descriptions_vec().await;
    info!("\n📖 Node descriptions:");
    for desc in descriptions.iter().take(3) {
        // Show first 3 for brevity
        info!("  🔧 {}: {}", desc.node_type, desc.description);
        if !desc.examples.is_empty() {
            info!("     Example: {}", desc.examples[0].name);
        }
    }

    // Get registry statistics
    let stats = node_registry.get_stats();
    info!(
        "\n📊 Registry stats: {} total nodes across {} categories",
        stats.total_nodes,
        stats.category_counts.len()
    );

    // Validate all nodes
    info!("\n🔍 Validating all nodes...");
    let validation_results = node_registry.validate_all().await;
    let mut valid_count = 0;
    let mut error_count = 0;

    for result in validation_results {
        if result.is_valid {
            valid_count += 1;
        } else {
            error_count += 1;
            info!("❌ {}: {:?}", result.node_type, result.errors);
        }
    }

    info!(
        "✅ Validation complete: {} valid, {} errors",
        valid_count, error_count
    );

    Ok(())
}

/// Demo 4: API server (optional)
#[allow(dead_code)]
async fn demo_api_server(
    execution_engine: Arc<ExecutionEngine>,
    node_registry: Arc<NodeRegistry>,
    state_manager: Arc<StateManager>,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("\n📋 Demo 4: Starting API Server");

    let _api_server = ApiServer::new(execution_engine, node_registry, state_manager);

    info!("🌐 API server starting on port 8080");
    info!("   Try these endpoints:");
    info!("   GET  http://localhost:8080/health");
    info!("   GET  http://localhost:8080/api/v1/nodes");
    info!("   POST http://localhost:8080/api/v1/workflows");

    // This will run indefinitely - comment out the .await to skip
    // api_server.serve(8080).await?;

    Ok(())
}
