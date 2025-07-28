//! Database query node for database operations

use crate::core::execution::ExecutionContext;
use crate::error::Result;
use crate::nodes::traits::{
    BaseNodeValidator, Node, NodeCapabilities, NodeDescription, NodeExample, NodeInput,
    NodeMetadata, NodeOutput, NodeSchema, NodeValidator, PropertySchema, ResourceRequirements,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Database query node for database operations
pub struct DatabaseQueryNode;

impl DatabaseQueryNode {
    /// Create a new database query node
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Node for DatabaseQueryNode {
    fn node_type(&self) -> &'static str {
        "database_query"
    }

    async fn validate_config(&self, config: &Value) -> Result<()> {
        BaseNodeValidator::validate_config(config, &self.config_schema())
    }

    async fn execute(
        &self,
        _context: &mut ExecutionContext,
        _input: NodeInput,
    ) -> Result<NodeOutput> {
        let start = std::time::Instant::now();

        // For now, return a mock response
        // In a real implementation, this would connect to the database
        let mock_result = json!({
            "rows": [],
            "affected_rows": 0,
            "query_time_ms": 15
        });

        let metadata = NodeMetadata::success()
            .with_duration(start.elapsed().as_millis() as u64)
            .with_records_processed(0);

        Ok(NodeOutput::with_metadata(mock_result, metadata))
    }

    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: "database_query".to_string(),
            description: "Executes SQL queries against a database".to_string(),
            inputs: self.input_schema(),
            outputs: self.output_schema(),
            config: self.config_schema(),
            examples: vec![NodeExample {
                name: "Select query".to_string(),
                description: "Execute a SELECT query".to_string(),
                config: json!({
                    "connection": "main_db",
                    "query": "SELECT * FROM users WHERE active = $1",
                    "params": [true]
                }),
                input: json!({}),
                output: json!({
                    "rows": [{"id": 1, "name": "John", "active": true}],
                    "affected_rows": 1,
                    "query_time_ms": 15
                }),
            }],
        }
    }

    fn config_schema(&self) -> NodeSchema {
        let mut schema = NodeSchema {
            required: vec!["connection".to_string(), "query".to_string()],
            ..Default::default()
        };

        let mut properties = HashMap::new();

        properties.insert(
            "connection".to_string(),
            PropertySchema {
                property_type: "string".to_string(),
                description: "Database connection name".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "query".to_string(),
            PropertySchema {
                property_type: "string".to_string(),
                description: "SQL query to execute".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        schema.properties = properties;
        schema
    }

    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities {
            supports_streaming: false,
            supports_batch: true,
            cacheable: false,
            has_side_effects: true,
            idempotent: false,
            resource_requirements: ResourceRequirements {
                memory_mb: Some(20),
                cpu_percent: Some(5.0),
                network_io: true,
                disk_io: true,
                external_dependencies: vec!["database".to_string()],
            },
        }
    }
}

impl Default for DatabaseQueryNode {
    fn default() -> Self {
        Self::new()
    }
}
