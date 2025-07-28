//! Control flow nodes for workflow orchestration

use crate::core::execution::ExecutionContext;
use crate::error::{NodeError, Result};
use crate::nodes::traits::{
    BaseNodeValidator, Node, NodeCapabilities, NodeDescription, NodeInput, NodeMetadata,
    NodeOutput, NodeSchema, NodeValidator,
};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Switch node for multi-branch conditional logic
pub struct SwitchNode;

impl SwitchNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Node for SwitchNode {
    fn node_type(&self) -> &'static str {
        "switch"
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

        let switch_value = input.get_config::<String>("on")?;
        let evaluated_value = context.evaluate_expression(&switch_value)?;

        // Get cases
        let cases = input
            .config
            .get("cases")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                crate::error::AutomataError::Node(NodeError::InvalidConfig {
                    node_type: "switch".to_string(),
                    message: "Missing 'cases' configuration".to_string(),
                })
            })?;

        // Find matching case
        let result = if let Some(case_value) = cases.get(&evaluated_value.to_string()) {
            case_value.clone()
        } else if let Some(default_value) = input.config.get("default") {
            default_value.clone()
        } else {
            Value::Null
        };

        let result_data = json!({
            "switch_value": evaluated_value,
            "result": result
        });

        let metadata = NodeMetadata::success().with_duration(start.elapsed().as_millis() as u64);

        Ok(NodeOutput::with_metadata(result_data, metadata))
    }

    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: "switch".to_string(),
            description: "Multi-way branching based on a value".to_string(),
            inputs: self.input_schema(),
            outputs: self.output_schema(),
            config: self.config_schema(),
            examples: vec![],
        }
    }

    fn config_schema(&self) -> NodeSchema {
        NodeSchema {
            required: vec!["on".to_string(), "cases".to_string()],
            ..Default::default()
        }
    }

    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities::default()
    }
}

/// ForEach node for iterating over arrays
pub struct ForEachNode;

impl ForEachNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Node for ForEachNode {
    fn node_type(&self) -> &'static str {
        "foreach"
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

        // Mock implementation - would iterate over items in real implementation
        let result_data = json!({
            "processed": 0,
            "results": []
        });

        let metadata = NodeMetadata::success().with_duration(start.elapsed().as_millis() as u64);

        Ok(NodeOutput::with_metadata(result_data, metadata))
    }

    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: "foreach".to_string(),
            description: "Iterates over array items".to_string(),
            inputs: self.input_schema(),
            outputs: self.output_schema(),
            config: self.config_schema(),
            examples: vec![],
        }
    }

    fn config_schema(&self) -> NodeSchema {
        NodeSchema::default()
    }

    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities::default()
    }
}

/// Parallel node for parallel execution
pub struct ParallelNode;

impl ParallelNode {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Node for ParallelNode {
    fn node_type(&self) -> &'static str {
        "parallel"
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

        // Mock implementation
        let result_data = json!({
            "completed": 0,
            "results": []
        });

        let metadata = NodeMetadata::success().with_duration(start.elapsed().as_millis() as u64);

        Ok(NodeOutput::with_metadata(result_data, metadata))
    }

    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: "parallel".to_string(),
            description: "Executes multiple operations in parallel".to_string(),
            inputs: self.input_schema(),
            outputs: self.output_schema(),
            config: self.config_schema(),
            examples: vec![],
        }
    }

    fn config_schema(&self) -> NodeSchema {
        NodeSchema::default()
    }

    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities::default()
    }
}

impl Default for SwitchNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ForEachNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ParallelNode {
    fn default() -> Self {
        Self::new()
    }
}
