//! Data transformer node for mapping and transforming data

use crate::core::execution::ExecutionContext;
use crate::error::{NodeError, Result};
use crate::nodes::traits::{
    BaseNodeValidator, Node, NodeCapabilities, NodeDescription, NodeExample, NodeInput,
    NodeMetadata, NodeOutput, NodeSchema, NodeValidator, PropertySchema, ResourceRequirements,
};
use async_trait::async_trait;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use tracing::debug;

/// Transformer node for data mapping and transformation
pub struct TransformerNode;

impl TransformerNode {
    /// Create a new transformer node
    pub fn new() -> Self {
        Self
    }

    /// Apply transformation mapping to data
    async fn apply_mapping(
        &self,
        data: &Value,
        mapping: &Value,
        context: &ExecutionContext,
    ) -> Result<Value> {
        match mapping {
            Value::Object(map) => {
                let mut result = Map::new();
                for (key, value) in map {
                    let transformed_value = self.transform_value(data, value, context).await?;
                    result.insert(key.clone(), transformed_value);
                }
                Ok(Value::Object(result))
            }
            _ => {
                // If mapping is not an object, treat it as a direct transformation
                self.transform_value(data, mapping, context).await
            }
        }
    }

    /// Transform a single value
    #[allow(clippy::only_used_in_recursion)]
    fn transform_value<'a>(
        &'a self,
        data: &'a Value,
        transform: &'a Value,
        context: &'a ExecutionContext,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value>> + Send + 'a>> {
        Box::pin(async move {
            match transform {
                Value::String(s) if s.starts_with('$') => {
                    // Expression - evaluate it
                    context.evaluate_expression(s)
                }
                Value::Object(obj) => {
                    // Nested object transformation
                    let mut result = Map::new();
                    for (key, value) in obj {
                        let transformed = self.transform_value(data, value, context).await?;
                        result.insert(key.clone(), transformed);
                    }
                    Ok(Value::Object(result))
                }
                Value::Array(arr) => {
                    // Transform each array element
                    let mut result = Vec::new();
                    for item in arr {
                        let transformed = self.transform_value(data, item, context).await?;
                        result.push(transformed);
                    }
                    Ok(Value::Array(result))
                }
                _ => {
                    // Literal value - return as is
                    Ok(transform.clone())
                }
            }
        })
    }

    /// Apply filter to data
    async fn apply_filter(
        &self,
        data: &Value,
        filter: &Value,
        context: &ExecutionContext,
    ) -> Result<Value> {
        if let Some(condition) = filter.get("condition") {
            let condition_str = condition.as_str().ok_or_else(|| {
                crate::error::AutomataError::Node(NodeError::InvalidConfig {
                    node_type: "transformer".to_string(),
                    message: "Filter condition must be a string expression".to_string(),
                })
            })?;

            // Create temporary context with current data as "item"
            let mut temp_context = context.clone();
            temp_context.set_node_output("item".to_string(), data.clone());

            let result = temp_context.evaluate_expression(condition_str)?;
            let passes_filter = match result {
                Value::Bool(b) => b,
                Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
                Value::String(s) => !s.is_empty(),
                _ => false,
            };

            if passes_filter {
                Ok(data.clone())
            } else {
                Ok(Value::Null)
            }
        } else {
            Ok(data.clone())
        }
    }

    /// Sort data array
    fn sort_data(&self, data: &Value, sort_config: &Value) -> Result<Value> {
        if let Value::Array(arr) = data {
            let mut sorted_arr = arr.clone();

            if let Some(sort_by) = sort_config.get("by").and_then(|v| v.as_str()) {
                let descending = sort_config
                    .get("order")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "desc")
                    .unwrap_or(false);

                sorted_arr.sort_by(|a, b| {
                    let a_val = a
                        .pointer(&format!("/{}", sort_by.replace('.', "/")))
                        .unwrap_or(&Value::Null);
                    let b_val = b
                        .pointer(&format!("/{}", sort_by.replace('.', "/")))
                        .unwrap_or(&Value::Null);

                    let comparison = match (a_val, b_val) {
                        (Value::Number(a_num), Value::Number(b_num)) => a_num
                            .as_f64()
                            .partial_cmp(&b_num.as_f64())
                            .unwrap_or(std::cmp::Ordering::Equal),
                        (Value::String(a_str), Value::String(b_str)) => a_str.cmp(b_str),
                        _ => std::cmp::Ordering::Equal,
                    };

                    if descending {
                        comparison.reverse()
                    } else {
                        comparison
                    }
                });
            }

            Ok(Value::Array(sorted_arr))
        } else {
            Ok(data.clone())
        }
    }

    /// Limit/paginate data
    fn limit_data(&self, data: &Value, limit_config: &Value) -> Result<Value> {
        if let Value::Array(arr) = data {
            let offset = limit_config
                .get("offset")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            let limit = limit_config
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(arr.len() as u64) as usize;

            let end = std::cmp::min(offset + limit, arr.len());
            let limited_arr = if offset < arr.len() {
                arr[offset..end].to_vec()
            } else {
                vec![]
            };

            Ok(Value::Array(limited_arr))
        } else {
            Ok(data.clone())
        }
    }
}

#[async_trait]
impl Node for TransformerNode {
    fn node_type(&self) -> &'static str {
        "transformer"
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

        // Get input data
        let data_to_transform = if let Some(input_expr) = input.config.get("input") {
            let input_str = input_expr.as_str().unwrap_or("$trigger.body");
            context.evaluate_expression(input_str)?
        } else {
            context.evaluate_expression("$trigger.body")?
        };

        debug!(data = %data_to_transform, "Transforming data");

        let mut result_data = data_to_transform;

        // Apply mapping if specified
        if let Some(mapping) = input.config.get("mapping") {
            result_data = self.apply_mapping(&result_data, mapping, context).await?;
        }

        // Apply filter if specified
        if let Some(filter) = input.config.get("filter") {
            if result_data.is_array() {
                // Filter array elements
                let filtered_items = if let Value::Array(arr) = &result_data {
                    let mut filtered = Vec::new();
                    for item in arr {
                        let filtered_item = self.apply_filter(item, filter, context).await?;
                        if !filtered_item.is_null() {
                            filtered.push(filtered_item);
                        }
                    }
                    filtered
                } else {
                    vec![]
                };
                result_data = Value::Array(filtered_items);
            } else {
                result_data = self.apply_filter(&result_data, filter, context).await?;
            }
        }

        // Apply sort if specified
        if let Some(sort_config) = input.config.get("sort") {
            result_data = self.sort_data(&result_data, sort_config)?;
        }

        // Apply limit if specified
        if let Some(limit_config) = input.config.get("limit") {
            result_data = self.limit_data(&result_data, limit_config)?;
        }

        let metadata = NodeMetadata::success().with_duration(start.elapsed().as_millis() as u64);

        Ok(NodeOutput::with_metadata(result_data, metadata))
    }

    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: "transformer".to_string(),
            description: "Transforms, filters, sorts, and manipulates data structures".to_string(),
            inputs: self.input_schema(),
            outputs: self.output_schema(),
            config: self.config_schema(),
            examples: vec![
                NodeExample {
                    name: "Simple field mapping".to_string(),
                    description: "Map input fields to output fields".to_string(),
                    config: json!({
                        "mapping": {
                            "user_id": "$trigger.body.id",
                            "user_name": "$trigger.body.name",
                            "email": "$trigger.body.email"
                        }
                    }),
                    input: json!({}),
                    output: json!({
                        "user_id": 123,
                        "user_name": "John Doe",
                        "email": "john@example.com"
                    }),
                },
                NodeExample {
                    name: "Array filtering and sorting".to_string(),
                    description: "Filter and sort an array of items".to_string(),
                    config: json!({
                        "input": "$trigger.body.items",
                        "filter": {
                            "condition": "$item.active == true"
                        },
                        "sort": {
                            "by": "created_at",
                            "order": "desc"
                        },
                        "limit": {
                            "offset": 0,
                            "limit": 10
                        }
                    }),
                    input: json!({}),
                    output: json!([
                        {"id": 2, "name": "Item 2", "active": true, "created_at": "2023-12-02"},
                        {"id": 1, "name": "Item 1", "active": true, "created_at": "2023-12-01"}
                    ]),
                },
            ],
        }
    }

    fn config_schema(&self) -> NodeSchema {
        let mut schema = NodeSchema::default();
        let mut properties = HashMap::new();

        properties.insert(
            "input".to_string(),
            PropertySchema {
                property_type: "string".to_string(),
                description: "Expression to get data to transform (default: $trigger.body)"
                    .to_string(),
                default: Some(json!("$trigger.body")),
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "mapping".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Field mapping configuration for transforming data structure"
                    .to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "filter".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Filter configuration for filtering data".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "sort".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Sort configuration for ordering data".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "limit".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "Limit/pagination configuration".to_string(),
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
            supports_streaming: true,
            supports_batch: true,
            cacheable: true,
            has_side_effects: false,
            idempotent: true,
            resource_requirements: ResourceRequirements {
                memory_mb: Some(10),
                cpu_percent: Some(3.0),
                network_io: false,
                disk_io: false,
                external_dependencies: vec![],
            },
        }
    }
}

impl Default for TransformerNode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::execution::ExecutionContext;
    use serde_json::json;
    use uuid::Uuid;

    fn create_test_context() -> ExecutionContext {
        ExecutionContext::new(
            Uuid::new_v4(),
            json!({
                "body": {
                    "id": 123,
                    "name": "John Doe",
                    "email": "john@example.com",
                    "items": [
                        {"id": 1, "name": "Item 1", "active": true, "value": 10},
                        {"id": 2, "name": "Item 2", "active": false, "value": 20},
                        {"id": 3, "name": "Item 3", "active": true, "value": 30}
                    ]
                }
            }),
        )
    }

    #[test]
    fn test_transformer_node_creation() {
        let node = TransformerNode::new();
        assert_eq!(node.node_type(), "transformer");
    }

    #[tokio::test]
    async fn test_simple_mapping() {
        let node = TransformerNode::new();
        let mut context = create_test_context();

        let input = NodeInput::new(
            json!({
                "mapping": {
                    "user_id": "$trigger.body.id",
                    "user_name": "$trigger.body.name"
                }
            }),
            json!({}),
        );

        let result = node.execute(&mut context, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        let data = &output.data;
        assert_eq!(data.get("user_id"), Some(&json!(123)));
        assert_eq!(data.get("user_name"), Some(&json!("John Doe")));
    }

    #[tokio::test]
    async fn test_array_filtering() {
        let node = TransformerNode::new();
        let mut context = create_test_context();

        let input = NodeInput::new(
            json!({
                "input": "$trigger.body.items",
                "filter": {
                    "condition": "$item.active"
                }
            }),
            json!({}),
        );

        let result = node.execute(&mut context, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        let data = output.data.as_array().unwrap();
        assert_eq!(data.len(), 2); // Only active items
    }
}
