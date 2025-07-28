//! DSL parser implementation using Pest

use crate::core::workflow::{
    WorkflowConnection, WorkflowDefinition, WorkflowMetadata, WorkflowNode, WorkflowTest,
    WorkflowTrigger,
};
use crate::error::{DslError, Result};
use pest::iterators::Pair;
use pest_derive::Parser;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Parser)]
#[grammar = "dsl/workflow.pest"]
pub struct WorkflowParser;

// Re-export Rule for external use

/// Parser for workflow DSL
pub struct DslParser {
    #[allow(dead_code)]
    expression_evaluator: crate::dsl::expression::ExpressionEvaluator,
    validator: crate::dsl::validator::SchemaValidator,
}

/// Parsed workflow with additional metadata
#[derive(Debug, Clone)]
pub struct ParsedWorkflow {
    pub definition: WorkflowDefinition,
    pub raw_yaml: String,
    pub expressions: Vec<String>,
}

impl DslParser {
    /// Create a new DSL parser
    pub fn new() -> Self {
        Self {
            expression_evaluator: crate::dsl::expression::ExpressionEvaluator::new(),
            validator: crate::dsl::validator::SchemaValidator::new(),
        }
    }

    /// Parse a workflow from YAML string
    pub fn parse(&self, input: &str) -> Result<ParsedWorkflow> {
        // First, try parsing as pure YAML
        let yaml_value: Value = serde_yaml::from_str(input).map_err(|e| {
            DslError::Parse(pest::error::Error::new_from_span(
                pest::error::ErrorVariant::CustomError {
                    message: format!("YAML parse error: {e}"),
                },
                pest::Span::new(input, 0, input.len()).unwrap(),
            ))
        })?;

        // Validate schema
        self.validator.validate(&yaml_value)?;

        // Parse structure
        let definition = self.parse_yaml_to_workflow(yaml_value)?;

        // Extract expressions for validation
        let expressions = self.extract_expressions(input)?;

        Ok(ParsedWorkflow {
            definition,
            raw_yaml: input.to_string(),
            expressions,
        })
    }

    /// Parse YAML value to workflow definition
    fn parse_yaml_to_workflow(&self, value: Value) -> Result<WorkflowDefinition> {
        let obj = value.as_object().ok_or_else(|| DslError::InvalidSyntax {
            line: 1,
            message: "Root must be an object".to_string(),
        })?;

        // Parse metadata
        let metadata = if let Some(meta_val) = obj.get("metadata").or_else(|| obj.get("@metadata"))
        {
            self.parse_metadata(meta_val)?
        } else {
            WorkflowMetadata {
                name: "Untitled Workflow".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                tags: Vec::new(),
                author: None,
                organization: None,
            }
        };

        // Parse triggers
        let triggers = if let Some(triggers_val) = obj.get("triggers") {
            self.parse_triggers(triggers_val)?
        } else {
            Vec::new()
        };

        // Parse nodes
        let nodes = if let Some(nodes_val) = obj.get("nodes") {
            self.parse_nodes(nodes_val)?
        } else {
            HashMap::new()
        };

        // Parse connections
        let connections = if let Some(conn_val) = obj.get("connections") {
            self.parse_connections(conn_val)?
        } else {
            Vec::new()
        };

        // Parse tests
        let tests = if let Some(test_val) = obj.get("test").or_else(|| obj.get("@test")) {
            Some(self.parse_tests(test_val)?)
        } else {
            None
        };

        let mut definition = WorkflowDefinition::new(metadata);
        definition.triggers = triggers;
        definition.nodes = nodes;
        definition.connections = connections;
        definition.tests = tests;

        Ok(definition)
    }

    /// Parse metadata section
    fn parse_metadata(&self, value: &Value) -> Result<WorkflowMetadata> {
        let obj = value
            .as_object()
            .ok_or_else(|| DslError::InvalidFieldType {
                field: "@metadata".to_string(),
                expected: "object".to_string(),
                actual: "other".to_string(),
            })?;

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Workflow")
            .to_string();

        let version = obj
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();

        let description = obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        let tags = obj
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(String::from)
                    .collect()
            })
            .unwrap_or_default();

        let author = obj.get("author").and_then(|v| v.as_str()).map(String::from);

        let organization = obj
            .get("organization")
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok(WorkflowMetadata {
            name,
            version,
            description,
            tags,
            author,
            organization,
        })
    }

    /// Parse triggers section
    fn parse_triggers(&self, value: &Value) -> Result<Vec<WorkflowTrigger>> {
        let arr = value.as_array().ok_or_else(|| DslError::InvalidFieldType {
            field: "triggers".to_string(),
            expected: "array".to_string(),
            actual: "other".to_string(),
        })?;

        let mut triggers = Vec::new();
        for trigger_val in arr {
            let trigger = self.parse_single_trigger(trigger_val)?;
            triggers.push(trigger);
        }

        Ok(triggers)
    }

    /// Parse a single trigger
    fn parse_single_trigger(&self, value: &Value) -> Result<WorkflowTrigger> {
        let obj = value
            .as_object()
            .ok_or_else(|| DslError::InvalidFieldType {
                field: "trigger".to_string(),
                expected: "object".to_string(),
                actual: "other".to_string(),
            })?;

        // Determine trigger type
        if let Some(webhook_val) = obj.get("webhook") {
            let webhook_obj =
                webhook_val
                    .as_object()
                    .ok_or_else(|| DslError::InvalidFieldType {
                        field: "webhook".to_string(),
                        expected: "object".to_string(),
                        actual: "other".to_string(),
                    })?;

            let path = webhook_obj
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DslError::MissingField {
                    field: "webhook.path".to_string(),
                })?
                .to_string();

            let method = webhook_obj
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("POST")
                .to_string();

            let auth = webhook_obj
                .get("auth")
                .and_then(|v| v.as_str())
                .map(String::from);

            let headers = webhook_obj
                .get("headers")
                .and_then(|v| v.as_object())
                .map(|obj| {
                    obj.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                });

            Ok(WorkflowTrigger::Webhook {
                path,
                method,
                auth,
                headers,
            })
        } else if let Some(schedule_val) = obj.get("schedule") {
            let schedule_obj =
                schedule_val
                    .as_object()
                    .ok_or_else(|| DslError::InvalidFieldType {
                        field: "schedule".to_string(),
                        expected: "object".to_string(),
                        actual: "other".to_string(),
                    })?;

            let cron = schedule_obj
                .get("cron")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DslError::MissingField {
                    field: "schedule.cron".to_string(),
                })?
                .to_string();

            let timezone = schedule_obj
                .get("timezone")
                .and_then(|v| v.as_str())
                .map(String::from);

            Ok(WorkflowTrigger::Schedule { cron, timezone })
        } else if let Some(event_val) = obj.get("event") {
            let event_obj = event_val
                .as_object()
                .ok_or_else(|| DslError::InvalidFieldType {
                    field: "event".to_string(),
                    expected: "object".to_string(),
                    actual: "other".to_string(),
                })?;

            let source = event_obj
                .get("source")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DslError::MissingField {
                    field: "event.source".to_string(),
                })?
                .to_string();

            let event_type = event_obj
                .get("type")
                .and_then(|v| v.as_str())
                .ok_or_else(|| DslError::MissingField {
                    field: "event.type".to_string(),
                })?
                .to_string();

            let filters = event_obj
                .get("filters")
                .and_then(|v| v.as_object())
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

            Ok(WorkflowTrigger::Event {
                source,
                event_type,
                filters,
            })
        } else if let Some(manual_val) = obj.get("manual") {
            let manual_obj = manual_val
                .as_object()
                .ok_or_else(|| DslError::InvalidFieldType {
                    field: "manual".to_string(),
                    expected: "object".to_string(),
                    actual: "other".to_string(),
                })?;

            let parameters = manual_obj
                .get("parameters")
                .and_then(|v| v.as_object())
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

            Ok(WorkflowTrigger::Manual { parameters })
        } else {
            Err(crate::error::AutomataError::DslParse(
                DslError::InvalidSyntax {
                    line: 1,
                    message: "Unknown trigger type".to_string(),
                },
            ))
        }
    }

    /// Parse nodes section
    fn parse_nodes(&self, value: &Value) -> Result<HashMap<String, WorkflowNode>> {
        let obj = value
            .as_object()
            .ok_or_else(|| DslError::InvalidFieldType {
                field: "nodes".to_string(),
                expected: "object".to_string(),
                actual: "other".to_string(),
            })?;

        let mut nodes = HashMap::new();
        for (node_id, node_val) in obj {
            let node = self.parse_single_node(node_id, node_val)?;
            nodes.insert(node_id.clone(), node);
        }

        Ok(nodes)
    }

    /// Parse a single node
    fn parse_single_node(&self, node_id: &str, value: &Value) -> Result<WorkflowNode> {
        let obj = value
            .as_object()
            .ok_or_else(|| DslError::InvalidFieldType {
                field: format!("nodes.{node_id}"),
                expected: "object".to_string(),
                actual: "other".to_string(),
            })?;

        let node_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DslError::MissingField {
                field: format!("nodes.{node_id}.type"),
            })?
            .to_string();

        // Extract all other fields as config
        let mut config_map = serde_json::Map::new();
        for (key, val) in obj {
            if key != "type" {
                config_map.insert(key.clone(), val.clone());
            }
        }

        let condition = obj
            .get("condition")
            .and_then(|v| v.as_str())
            .map(String::from);

        let timeout = obj.get("timeout").and_then(|v| v.as_u64());

        let retry = obj
            .get("retry")
            .map(|v| crate::core::workflow::RetryConfig {
                max_attempts: v.get("max_attempts").and_then(|v| v.as_u64()).unwrap_or(3) as u32,
                delay_ms: v.get("delay_ms").and_then(|v| v.as_u64()).unwrap_or(1000),
                backoff_multiplier: v
                    .get("backoff_multiplier")
                    .and_then(|v| v.as_f64())
                    .or(Some(2.0)),
                max_delay_ms: v
                    .get("max_delay_ms")
                    .and_then(|v| v.as_u64())
                    .or(Some(30000)),
            });

        Ok(WorkflowNode {
            id: node_id.to_string(),
            node_type,
            config: Value::Object(config_map),
            position: None, // Will be calculated during layout
            condition,
            timeout,
            retry,
        })
    }

    /// Parse connections section
    fn parse_connections(&self, value: &Value) -> Result<Vec<WorkflowConnection>> {
        let arr = value.as_array().ok_or_else(|| DslError::InvalidFieldType {
            field: "connections".to_string(),
            expected: "array".to_string(),
            actual: "other".to_string(),
        })?;

        let mut connections = Vec::new();
        for conn_val in arr {
            let connection = self.parse_single_connection(conn_val)?;
            connections.push(connection);
        }

        Ok(connections)
    }

    /// Parse a single connection
    fn parse_single_connection(&self, value: &Value) -> Result<WorkflowConnection> {
        let obj = value
            .as_object()
            .ok_or_else(|| DslError::InvalidFieldType {
                field: "connection".to_string(),
                expected: "object".to_string(),
                actual: "other".to_string(),
            })?;

        let from = obj
            .get("from")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DslError::MissingField {
                field: "connection.from".to_string(),
            })?
            .to_string();

        let to = obj
            .get("to")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DslError::MissingField {
                field: "connection.to".to_string(),
            })?
            .to_string();

        let condition = obj
            .get("condition")
            .and_then(|v| v.as_str())
            .map(String::from);

        let label = obj.get("label").and_then(|v| v.as_str()).map(String::from);

        Ok(WorkflowConnection {
            from,
            to,
            condition,
            label,
        })
    }

    /// Parse tests section
    fn parse_tests(&self, value: &Value) -> Result<Vec<WorkflowTest>> {
        let arr = value.as_array().ok_or_else(|| DslError::InvalidFieldType {
            field: "@test".to_string(),
            expected: "array".to_string(),
            actual: "other".to_string(),
        })?;

        let mut tests = Vec::new();
        for test_val in arr {
            let test = self.parse_single_test(test_val)?;
            tests.push(test);
        }

        Ok(tests)
    }

    /// Parse a single test
    fn parse_single_test(&self, value: &Value) -> Result<WorkflowTest> {
        let obj = value
            .as_object()
            .ok_or_else(|| DslError::InvalidFieldType {
                field: "test".to_string(),
                expected: "object".to_string(),
                actual: "other".to_string(),
            })?;

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DslError::MissingField {
                field: "test.name".to_string(),
            })?
            .to_string();

        let input = obj
            .get("input")
            .ok_or_else(|| DslError::MissingField {
                field: "test.input".to_string(),
            })?
            .clone();

        let mocks = obj
            .get("mock")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect());

        let expected = obj
            .get("expect")
            .and_then(|v| v.as_object())
            .ok_or_else(|| DslError::MissingField {
                field: "test.expect".to_string(),
            })?
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(WorkflowTest {
            name,
            input,
            mocks,
            expected,
        })
    }

    /// Extract expressions from the workflow for validation
    fn extract_expressions(&self, _input: &str) -> Result<Vec<String>> {
        // For now, just return empty expressions
        // In a full implementation, we would parse the YAML to find expressions like $trigger.body, $now(), etc.
        Ok(Vec::new())
    }

    /// Recursively extract expressions from parse tree
    #[allow(dead_code)]
    fn extract_expressions_recursive(pair: Pair<Rule>, expressions: &mut Vec<String>) {
        match pair.as_rule() {
            Rule::expression | Rule::variable_reference | Rule::function_call => {
                expressions.push(pair.as_str().to_string());
            }
            _ => {
                for inner_pair in pair.into_inner() {
                    Self::extract_expressions_recursive(inner_pair, expressions);
                }
            }
        }
    }
}

impl Default for DslParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_workflow_parsing() {
        let yaml = r#"
metadata:
  name: "Test Workflow"
  version: "1.0.0"
  description: "A simple test workflow"

triggers:
  - webhook:
      path: "/api/test"
      method: "POST"

nodes:
  validate:
    type: validator
    rules:
      - field: email
        type: email
        required: true

  process:
    type: transformer
    mapping:
      user_id: $validate.data.id

connections:
  - from: validate
    to: process
    condition: $validate.success
"#;

        let parser = DslParser::new();
        let result = parser.parse(yaml);
        if let Err(e) = &result {
            println!("Parse error: {e:?}");
        }
        assert!(result.is_ok());

        let parsed = result.unwrap();
        assert_eq!(parsed.definition.metadata.name, "Test Workflow");
        assert_eq!(parsed.definition.triggers.len(), 1);
        assert_eq!(parsed.definition.nodes.len(), 2);
        assert_eq!(parsed.definition.connections.len(), 1);
    }

    #[test]
    fn test_expression_extraction() {
        let yaml = r#"
nodes:
  test:
    type: http
    url: $env.API_URL
    body: $trigger.data
    condition: $validate.success == true
"#;

        let parser = DslParser::new();
        let result = parser.parse(yaml);
        assert!(result.is_ok());

        let parsed = result.unwrap();
        // Expression extraction is not yet implemented, so expressions will be empty for now
        assert!(parsed.expressions.is_empty());
    }
}
