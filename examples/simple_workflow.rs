//! Simple workflow example

use automata::{
    core::engine::{ExecutionEngine, ExecutionEngineConfig},
    dsl::DslParser,
    nodes::NodeRegistry,
    state::StateManager,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create workflow definition
    let yaml_workflow = r#"
@metadata:
  name: "Simple Test Workflow"
  version: "1.0.0"
  description: "A simple test workflow"

triggers:
  - webhook:
      path: "/api/test"
      method: "POST"

nodes:
  validate_input:
    type: validator
    rules:
      - field: email
        type: string
        required: true
        email: true

  transform_data:
    type: transformer
    mapping:
      user_email: $validate_input.data.email
      timestamp: $now()

connections:
  - from: trigger
    to: validate_input
  - from: validate_input
    to: transform_data
    condition: $validate_input.valid
"#;

    // Parse workflow
    let parser = DslParser::new();
    let parsed_workflow = parser.parse(yaml_workflow)?;

    println!("Workflow parsed successfully!");
    println!("Name: {}", parsed_workflow.definition.metadata.name);
    println!("Nodes: {}", parsed_workflow.definition.nodes.len());
    println!(
        "Connections: {}",
        parsed_workflow.definition.connections.len()
    );

    // Create execution engine
    let node_registry = Arc::new(NodeRegistry::new());
    let state_manager = Arc::new(StateManager::new());
    let engine_config = ExecutionEngineConfig::default();

    let _execution_engine = ExecutionEngine::new(node_registry, state_manager, engine_config);

    println!("Execution engine created successfully!");

    Ok(())
}
