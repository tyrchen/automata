//! Conditional node for branching logic

use crate::core::execution::ExecutionContext;
use crate::error::Result;
use crate::nodes::traits::{
    BaseNodeValidator, Node, NodeCapabilities, NodeDescription, NodeExample, NodeInput,
    NodeMetadata, NodeOutput, NodeSchema, NodeValidator, PropertySchema, ResourceRequirements,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Conditional node for branching workflow logic
pub struct ConditionalNode;

impl ConditionalNode {
    /// Create a new conditional node
    pub fn new() -> Self {
        Self
    }

    /// Evaluate condition expression
    fn evaluate_condition(&self, condition: &str, context: &ExecutionContext) -> Result<bool> {
        let result = context.evaluate_expression(condition)?;

        // Convert result to boolean
        let condition_result = match result {
            Value::Bool(b) => b,
            Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
            Value::Null => false,
        };

        Ok(condition_result)
    }
}

#[async_trait]
impl Node for ConditionalNode {
    fn node_type(&self) -> &'static str {
        "conditional"
    }

    async fn validate_config(&self, config: &Value) -> Result<()> {
        BaseNodeValidator::validate_config(config, &self.config_schema())
    }

    async fn execute(
        &self,
        context: &mut ExecutionContext,
        input: NodeInput,
    ) -> Result<NodeOutput> {
        let start = std::time::Instant::now();

        // Get condition expression
        let condition = input.get_config::<String>("condition")?;

        // Evaluate condition
        let condition_result = self.evaluate_condition(&condition, context)?;

        // Get appropriate output based on condition
        let output_data = if condition_result {
            input
                .get_config_optional::<Value>("then")?
                .unwrap_or(json!(true))
        } else {
            input
                .get_config_optional::<Value>("else")?
                .unwrap_or(json!(false))
        };

        let result_data = json!({
            "condition": condition_result,
            "result": output_data
        });

        let metadata = NodeMetadata::success().with_duration(start.elapsed().as_millis() as u64);

        Ok(NodeOutput::with_metadata(result_data, metadata))
    }

    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: "conditional".to_string(),
            description: "Evaluates conditions and returns different outputs based on the result"
                .to_string(),
            inputs: self.input_schema(),
            outputs: self.output_schema(),
            config: self.config_schema(),
            examples: vec![NodeExample {
                name: "Simple boolean condition".to_string(),
                description: "Check if a user is active".to_string(),
                config: json!({
                    "condition": "$trigger.body.user.active == true",
                    "then": {"status": "proceed"},
                    "else": {"status": "blocked"}
                }),
                input: json!({}),
                output: json!({
                    "condition": true,
                    "result": {"status": "proceed"}
                }),
            }],
        }
    }

    fn config_schema(&self) -> NodeSchema {
        let mut schema = NodeSchema {
            required: vec!["condition".to_string()],
            ..Default::default()
        };

        let mut properties = HashMap::new();

        properties.insert(
            "condition".to_string(),
            PropertySchema {
                property_type: "string".to_string(),
                description: "Boolean expression to evaluate".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "then".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Value to return when condition is true".to_string(),
                default: Some(json!(true)),
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "else".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Value to return when condition is false".to_string(),
                default: Some(json!(false)),
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
            cacheable: true,
            has_side_effects: false,
            idempotent: true,
            resource_requirements: ResourceRequirements {
                memory_mb: Some(1),
                cpu_percent: Some(1.0),
                network_io: false,
                disk_io: false,
                external_dependencies: vec![],
            },
        }
    }
}

impl Default for ConditionalNode {
    fn default() -> Self {
        Self::new()
    }
}
