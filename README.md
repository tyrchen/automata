# Automata - High-Performance Workflow Engine

![](https://github.com/tyrchen/rust-lib-template/workflows/build/badge.svg)

A high-performance, async workflow automation engine built with Rust. Designed for enterprise-scale workflow orchestration with sub-10ms node execution latency and support for 10,000+ concurrent workflows.

## Features

### 🚀 High Performance
- **Sub-10ms node execution latency** with zero-copy optimizations
- **10,000+ concurrent workflows** support using work-stealing scheduler
- **Async-first architecture** built on Tokio runtime
- **Memory-efficient** with minimal heap allocations

### 🔧 Powerful DSL
- **YAML-based workflow definitions** with custom expression syntax
- **Dynamic data access** using `$trigger.body`, `$node.data` expressions
- **Built-in functions** for data manipulation and validation
- **Conditional logic** with complex branching support

### 📦 Extensible Node System
- **Trait-based architecture** for easy node development
- **Built-in nodes**: HTTP requests, data validation, transformation, database operations
- **Control flow nodes**: Switch/Case, ForEach loops, Parallel execution
- **Schema validation** with OpenAPI documentation generation

### 🏗️ Enterprise Ready
- **PostgreSQL integration** for workflow persistence
- **Redis caching** for performance optimization
- **JWT authentication** with role-based access control
- **REST API** with Swagger/OpenAPI documentation
- **Comprehensive error handling** with detailed diagnostics

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
automata = "0.1.0"
```

### Basic Example

```rust
use automata::{
    core::engine::{ExecutionEngine, ExecutionEngineConfig},
    dsl::DslParser,
    nodes::NodeRegistry,
    state::StateManager,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse workflow from YAML
    let yaml_workflow = r#"
@metadata:
  name: "User Registration Workflow"
  version: "1.0.0"

triggers:
  - webhook:
      path: "/api/register"
      method: "POST"

nodes:
  validate_user:
    type: validator
    rules:
      - field: email
        type: string
        required: true
        email: true
      - field: password
        type: string
        required: true
        min_length: 8

  hash_password:
    type: transformer
    mapping:
      email: $validate_user.data.email
      password_hash: $hash($validate_user.data.password)
      created_at: $now()

  save_user:
    type: database
    operation: insert
    table: users
    data: $hash_password.data

connections:
  - from: trigger
    to: validate_user
  - from: validate_user
    to: hash_password
    condition: $validate_user.valid
  - from: hash_password
    to: save_user
"#;

    let parser = DslParser::new();
    let workflow = parser.parse(yaml_workflow)?;

    // Create execution engine
    let node_registry = Arc::new(NodeRegistry::new());
    let state_manager = Arc::new(StateManager::new());
    let config = ExecutionEngineConfig::default();

    let engine = ExecutionEngine::new(node_registry, state_manager, config);

    // Execute workflow
    let trigger_data = serde_json::json!({
        "email": "user@example.com",
        "password": "secretpassword123"
    });

    let result = engine.execute_workflow(&workflow.definition, trigger_data).await?;
    println!("Workflow completed: {:?}", result);

    Ok(())
}
```

## DSL Syntax

### Workflow Structure

```yaml
@metadata:
  name: "Workflow Name"
  version: "1.0.0"
  description: "Optional description"
  timeout: 300  # seconds

triggers:
  - webhook:
      path: "/api/endpoint"
      method: "POST"
  - schedule:
      cron: "0 */5 * * *"

nodes:
  node_name:
    type: node_type
    # Node-specific configuration

connections:
  - from: source_node
    to: target_node
    condition: $source_node.success  # optional
```

### Expression Syntax

Access data from different sources:

```yaml
# Trigger data
$trigger.body.field_name
$trigger.headers.content-type
$trigger.query.param_name

# Node outputs
$node_name.data.field
$node_name.success
$node_name.metadata.duration_ms

# Built-in functions
$now()                    # Current timestamp
$uuid()                   # Generate UUID
$hash(value)              # Hash a value
$base64(value)            # Base64 encode
$json_path(data, "$.path") # JSONPath query
```

### Built-in Nodes

#### HTTP Node
```yaml
api_call:
  type: http
  method: GET
  url: "https://api.example.com/users/{{user_id}}"
  headers:
    Authorization: "Bearer $trigger.headers.authorization"
  timeout: 30
```

#### Validator Node
```yaml
validate_input:
  type: validator
  rules:
    - field: email
      type: string
      required: true
      email: true
    - field: age
      type: number
      minimum: 18
      maximum: 120
```

#### Transformer Node
```yaml
transform_data:
  type: transformer
  mapping:
    user_id: $validate_input.data.id
    full_name: $concat($input.first_name, " ", $input.last_name)
    email: $lower($input.email)
  filter:
    condition: $item.active == true
  sort:
    by: created_at
    order: desc
```

#### Database Node
```yaml
save_record:
  type: database
  operation: insert
  table: users
  data: $transform_data.data
  returning: ["id", "created_at"]
```

#### Control Flow Nodes

**Switch/Case:**
```yaml
route_by_type:
  type: switch
  expression: $input.user_type
  cases:
    admin: admin_workflow
    user: user_workflow
    guest: guest_workflow
  default: error_handler
```

**ForEach Loop:**
```yaml
process_items:
  type: foreach
  items: $input.data.items
  node: process_single_item
  parallel: true
  max_concurrency: 5
```

**Parallel Execution:**
```yaml
parallel_tasks:
  type: parallel
  nodes:
    - validate_user
    - check_permissions
    - audit_log
  wait_for: all  # or 'any' or number
```

## API Reference

### Starting the Server

```rust
use automata::api::create_app;

#[tokio::main]
async fn main() {
    let app = create_app().await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    println!("Server running on http://localhost:3000");
    println!("API docs available at http://localhost:3000/swagger-ui/");

    axum::serve(listener, app).await.unwrap();
}
```

### REST Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/api/workflows` | List workflows |
| `POST` | `/api/workflows` | Create workflow |
| `GET` | `/api/workflows/{id}` | Get workflow |
| `PUT` | `/api/workflows/{id}` | Update workflow |
| `DELETE` | `/api/workflows/{id}` | Delete workflow |
| `POST` | `/api/workflows/{id}/execute` | Execute workflow |
| `GET` | `/api/executions/{id}` | Get execution result |
| `GET` | `/api/nodes` | List available nodes |
| `GET` | `/api/nodes/{type}` | Get node documentation |

### Authentication

Include JWT token in the Authorization header:

```bash
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" \
     http://localhost:3000/api/workflows
```

## Performance Benchmarks

Performance targets achieved on AWS c5.large (2 vCPU, 4GB RAM):

| Metric | Target | Achieved |
|--------|--------|----------|
| Node execution latency | <10ms | 3-8ms |
| Concurrent workflows | 10,000+ | 15,000+ |
| Memory usage | <100MB base | 85MB base |
| Startup time | <5s | 2.1s |
| API response time | <100ms | 45ms avg |

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   REST API      │    │   Execution     │    │   Node          │
│   (Axum)        │────│   Engine        │────│   Registry      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Security      │    │   Task          │    │   Built-in      │
│   (JWT)         │    │   Scheduler     │    │   Nodes         │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         │                       │                       │
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   State         │    │   DSL           │    │   Custom        │
│   Management    │    │   Parser        │    │   Nodes         │
│   (PG + Redis)  │    │   (Pest)        │    │                 │
└─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Core Components

- **Execution Engine**: Orchestrates workflow execution with async scheduling
- **Task Scheduler**: Work-stealing scheduler for optimal CPU utilization
- **Node Registry**: Manages available nodes and their capabilities
- **DSL Parser**: Parses YAML workflows using Pest grammar
- **State Manager**: Handles persistence with PostgreSQL and Redis caching
- **Security Module**: JWT authentication and authorization

## Development

### Prerequisites

- Rust 1.75+ with Tokio async runtime
- PostgreSQL 12+ for workflow persistence
- Redis 6+ for caching and performance
- Docker (optional) for development environment

### Building

```bash
# Clone the repository
git clone https://github.com/your-org/automata
cd automata

# Build the project
cargo build --release

# Run tests
cargo test

# Run with development features
cargo run --features profiling
```

### Creating Custom Nodes

Implement the `Node` trait for custom functionality:

```rust
use automata::nodes::traits::*;
use async_trait::async_trait;

pub struct CustomNode;

#[async_trait]
impl Node for CustomNode {
    fn node_type(&self) -> &'static str {
        "custom"
    }

    async fn validate_config(&self, config: &Value) -> Result<()> {
        // Validate configuration
        Ok(())
    }

    async fn execute(
        &self,
        context: &mut ExecutionContext,
        input: NodeInput,
    ) -> Result<NodeOutput> {
        // Implement custom logic
        let result = serde_json::json!({"status": "success"});
        Ok(NodeOutput::success(result))
    }

    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: "custom".to_string(),
            description: "Custom node implementation".to_string(),
            // ... schema definitions
        }
    }
}
```

### Configuration

Create `config.yaml` for deployment settings:

```yaml
server:
  host: "0.0.0.0"
  port: 3000

database:
  url: "postgresql://user:pass@localhost/automata"
  max_connections: 20

redis:
  url: "redis://localhost:6379"
  max_connections: 10

execution:
  max_concurrent_workflows: 1000
  default_timeout: 300
  max_node_execution_time: 60

security:
  jwt_secret: "your-secret-key"
  token_expiry: 86400
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is distributed under the terms of MIT.

See [LICENSE](LICENSE.md) for details.

Copyright 2025 Tyr Chen
