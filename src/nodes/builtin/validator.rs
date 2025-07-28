//! Validator node for data validation

use crate::core::execution::ExecutionContext;
use crate::error::{NodeError, Result, ValidationError};
use crate::nodes::traits::{
    BaseNodeValidator, Node, NodeCapabilities, NodeDescription, NodeExample, NodeInput,
    NodeMetadata, NodeOutput, NodeSchema, NodeValidator, PropertySchema, ResourceRequirements,
};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::debug;

/// Validator node for validating data against rules
pub struct ValidatorNode;

impl ValidatorNode {
    /// Create a new validator node
    pub fn new() -> Self {
        Self
    }

    /// Validate a single field against rules
    fn validate_field(
        &self,
        field_name: &str,
        value: &Value,
        rules: &[ValidationRule],
    ) -> Vec<String> {
        let mut errors = Vec::new();

        for rule in rules {
            if let Err(error) = self.apply_rule(field_name, value, rule) {
                errors.push(error.to_string());
            }
        }

        errors
    }

    /// Apply a single validation rule
    fn apply_rule(&self, field_name: &str, value: &Value, rule: &ValidationRule) -> Result<()> {
        match rule {
            ValidationRule::Required => {
                if value.is_null() {
                    return Err(crate::error::AutomataError::Validation(
                        ValidationError::RequiredFieldMissing {
                            field: field_name.to_string(),
                        },
                    ));
                }
            }
            ValidationRule::Type(expected_type) => {
                if !self.check_type(value, expected_type) {
                    return Err(crate::error::AutomataError::Validation(
                        ValidationError::InvalidValue {
                            field: field_name.to_string(),
                            message: format!("Expected type {expected_type}, got {value:?}"),
                        },
                    ));
                }
            }
            ValidationRule::MinLength(min_len) => {
                let actual_len = match value {
                    Value::String(s) => s.len(),
                    Value::Array(arr) => arr.len(),
                    _ => 0,
                };
                if actual_len < *min_len {
                    return Err(crate::error::AutomataError::Validation(
                        ValidationError::ValueOutOfRange {
                            field: field_name.to_string(),
                            value: actual_len.to_string(),
                            min: min_len.to_string(),
                            max: "∞".to_string(),
                        },
                    ));
                }
            }
            ValidationRule::MaxLength(max_len) => {
                let actual_len = match value {
                    Value::String(s) => s.len(),
                    Value::Array(arr) => arr.len(),
                    _ => 0,
                };
                if actual_len > *max_len {
                    return Err(crate::error::AutomataError::Validation(
                        ValidationError::ValueOutOfRange {
                            field: field_name.to_string(),
                            value: actual_len.to_string(),
                            min: "0".to_string(),
                            max: max_len.to_string(),
                        },
                    ));
                }
            }
            ValidationRule::Minimum(min_val) => {
                if let Some(num) = value.as_f64() {
                    if num < *min_val {
                        return Err(crate::error::AutomataError::Validation(
                            ValidationError::ValueOutOfRange {
                                field: field_name.to_string(),
                                value: num.to_string(),
                                min: min_val.to_string(),
                                max: "∞".to_string(),
                            },
                        ));
                    }
                }
            }
            ValidationRule::Maximum(max_val) => {
                if let Some(num) = value.as_f64() {
                    if num > *max_val {
                        return Err(crate::error::AutomataError::Validation(
                            ValidationError::ValueOutOfRange {
                                field: field_name.to_string(),
                                value: num.to_string(),
                                min: "-∞".to_string(),
                                max: max_val.to_string(),
                            },
                        ));
                    }
                }
            }
            ValidationRule::Pattern(pattern) => {
                if let Some(string_val) = value.as_str() {
                    let regex = Regex::new(pattern).map_err(|_| {
                        crate::error::AutomataError::Validation(ValidationError::InvalidValue {
                            field: field_name.to_string(),
                            message: "Invalid regex pattern".to_string(),
                        })
                    })?;

                    if !regex.is_match(string_val) {
                        return Err(crate::error::AutomataError::Validation(
                            ValidationError::InvalidValue {
                                field: field_name.to_string(),
                                message: format!("Does not match pattern: {pattern}"),
                            },
                        ));
                    }
                }
            }
            ValidationRule::Email => {
                if let Some(string_val) = value.as_str() {
                    let email_regex =
                        Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();
                    if !email_regex.is_match(string_val) {
                        return Err(crate::error::AutomataError::Validation(
                            ValidationError::InvalidValue {
                                field: field_name.to_string(),
                                message: "Invalid email format".to_string(),
                            },
                        ));
                    }
                }
            }
            ValidationRule::Url => {
                if let Some(string_val) = value.as_str() {
                    let url_regex = Regex::new(r"^https?://.*").unwrap();
                    if !url_regex.is_match(string_val) {
                        return Err(crate::error::AutomataError::Validation(
                            ValidationError::InvalidValue {
                                field: field_name.to_string(),
                                message: "Invalid URL format".to_string(),
                            },
                        ));
                    }
                }
            }
            ValidationRule::OneOf(allowed_values) => {
                if !allowed_values.contains(value) {
                    return Err(crate::error::AutomataError::Validation(
                        ValidationError::InvalidValue {
                            field: field_name.to_string(),
                            message: format!("Must be one of: {allowed_values:?}"),
                        },
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if value matches expected type
    fn check_type(&self, value: &Value, expected_type: &str) -> bool {
        match expected_type {
            "string" => value.is_string(),
            "number" => value.is_number(),
            "integer" => value.as_i64().is_some(),
            "boolean" => value.is_boolean(),
            "array" => value.is_array(),
            "object" => value.is_object(),
            "null" => value.is_null(),
            _ => true, // Unknown types pass
        }
    }

    /// Parse validation rules from configuration
    fn parse_rules(&self, rules_config: &Value) -> Result<Vec<FieldValidation>> {
        let rules_array = rules_config.as_array().ok_or_else(|| {
            crate::error::AutomataError::Node(NodeError::InvalidConfig {
                node_type: "validator".to_string(),
                message: "Rules must be an array".to_string(),
            })
        })?;

        let mut field_validations = Vec::new();

        for rule_config in rules_array {
            let rule_obj = rule_config.as_object().ok_or_else(|| {
                crate::error::AutomataError::Node(NodeError::InvalidConfig {
                    node_type: "validator".to_string(),
                    message: "Each rule must be an object".to_string(),
                })
            })?;

            let field_name = rule_obj
                .get("field")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    crate::error::AutomataError::Node(NodeError::InvalidConfig {
                        node_type: "validator".to_string(),
                        message: "Rule must have 'field' property".to_string(),
                    })
                })?
                .to_string();

            let mut rules = Vec::new();

            // Required
            if rule_obj
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                rules.push(ValidationRule::Required);
            }

            // Type
            if let Some(type_str) = rule_obj.get("type").and_then(|v| v.as_str()) {
                rules.push(ValidationRule::Type(type_str.to_string()));
            }

            // Min/Max length
            if let Some(min_len) = rule_obj.get("min_length").and_then(|v| v.as_u64()) {
                rules.push(ValidationRule::MinLength(min_len as usize));
            }

            if let Some(max_len) = rule_obj.get("max_length").and_then(|v| v.as_u64()) {
                rules.push(ValidationRule::MaxLength(max_len as usize));
            }

            // Min/Max value
            if let Some(min_val) = rule_obj.get("minimum").and_then(|v| v.as_f64()) {
                rules.push(ValidationRule::Minimum(min_val));
            }

            if let Some(max_val) = rule_obj.get("maximum").and_then(|v| v.as_f64()) {
                rules.push(ValidationRule::Maximum(max_val));
            }

            // Pattern
            if let Some(pattern) = rule_obj.get("pattern").and_then(|v| v.as_str()) {
                rules.push(ValidationRule::Pattern(pattern.to_string()));
            }

            // Special formats
            if rule_obj
                .get("email")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                rules.push(ValidationRule::Email);
            }

            if rule_obj
                .get("url")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                rules.push(ValidationRule::Url);
            }

            // One of
            if let Some(one_of) = rule_obj.get("one_of").and_then(|v| v.as_array()) {
                rules.push(ValidationRule::OneOf(one_of.clone()));
            }

            field_validations.push(FieldValidation { field_name, rules });
        }

        Ok(field_validations)
    }
}

/// Validation rule types
#[derive(Debug, Clone)]
enum ValidationRule {
    Required,
    Type(String),
    MinLength(usize),
    MaxLength(usize),
    Minimum(f64),
    Maximum(f64),
    Pattern(String),
    Email,
    Url,
    OneOf(Vec<Value>),
}

/// Field validation configuration
#[derive(Debug, Clone)]
struct FieldValidation {
    field_name: String,
    rules: Vec<ValidationRule>,
}

#[async_trait]
impl Node for ValidatorNode {
    fn node_type(&self) -> &'static str {
        "validator"
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

        // Get input data to validate
        let data_to_validate = if let Some(input_expr) = input.config.get("input") {
            let input_str = input_expr.as_str().unwrap_or("$trigger.body");
            context.evaluate_expression(input_str)?
        } else {
            // Default to trigger body
            context.evaluate_expression("$trigger.body")?
        };

        debug!(data = %data_to_validate, "Validating data");

        // Parse validation rules
        let rules_config = input.config.get("rules").ok_or_else(|| {
            crate::error::AutomataError::Node(NodeError::InvalidConfig {
                node_type: "validator".to_string(),
                message: "Missing required 'rules' configuration".to_string(),
            })
        })?;

        let field_validations = self.parse_rules(rules_config)?;

        // Perform validation
        let mut all_errors = Vec::new();
        let validated_data = data_to_validate.clone();

        if let Some(data_obj) = data_to_validate.as_object() {
            for field_validation in &field_validations {
                let field_value = data_obj
                    .get(&field_validation.field_name)
                    .unwrap_or(&Value::Null);

                let field_errors = self.validate_field(
                    &field_validation.field_name,
                    field_value,
                    &field_validation.rules,
                );

                all_errors.extend(field_errors);
            }
        } else {
            // If not an object, validate the entire value against the first rule set
            if let Some(first_validation) = field_validations.first() {
                let field_errors = self.validate_field(
                    &first_validation.field_name,
                    &data_to_validate,
                    &first_validation.rules,
                );
                all_errors.extend(field_errors);
            }
        }

        let is_valid = all_errors.is_empty();

        // Create result
        let result_data = json!({
            "valid": is_valid,
            "data": validated_data,
            "errors": all_errors
        });

        let metadata = if is_valid {
            NodeMetadata::success()
        } else {
            NodeMetadata::failed()
        }
        .with_duration(start.elapsed().as_millis() as u64)
        .with_records_processed(field_validations.len() as u64);

        Ok(NodeOutput::with_metadata(result_data, metadata))
    }

    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: "validator".to_string(),
            description: "Validates data against specified rules and constraints".to_string(),
            inputs: self.input_schema(),
            outputs: self.output_schema(),
            config: self.config_schema(),
            examples: vec![
                NodeExample {
                    name: "Email validation".to_string(),
                    description: "Validate email format and requirement".to_string(),
                    config: json!({
                        "input": "$trigger.body",
                        "rules": [
                            {
                                "field": "email",
                                "type": "string",
                                "required": true,
                                "email": true
                            }
                        ]
                    }),
                    input: json!({}),
                    output: json!({
                        "valid": true,
                        "data": {"email": "user@example.com"},
                        "errors": []
                    }),
                },
                NodeExample {
                    name: "Complex validation".to_string(),
                    description: "Multiple field validation with different rules".to_string(),
                    config: json!({
                        "rules": [
                            {
                                "field": "name",
                                "type": "string",
                                "required": true,
                                "min_length": 2,
                                "max_length": 50
                            },
                            {
                                "field": "age",
                                "type": "number",
                                "required": true,
                                "minimum": 0,
                                "maximum": 120
                            },
                            {
                                "field": "status",
                                "type": "string",
                                "one_of": ["active", "inactive", "pending"]
                            }
                        ]
                    }),
                    input: json!({}),
                    output: json!({
                        "valid": true,
                        "data": {"name": "John", "age": 30, "status": "active"},
                        "errors": []
                    }),
                },
            ],
        }
    }

    fn config_schema(&self) -> NodeSchema {
        let mut schema = NodeSchema {
            required: vec!["rules".to_string()],
            ..Default::default()
        };

        let mut properties = HashMap::new();

        properties.insert(
            "input".to_string(),
            PropertySchema {
                property_type: "string".to_string(),
                description: "Expression to get data to validate (default: $trigger.body)"
                    .to_string(),
                default: Some(json!("$trigger.body")),
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "rules".to_string(),
            PropertySchema {
                property_type: "array".to_string(),
                description: "Array of validation rules for different fields".to_string(),
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

    fn output_schema(&self) -> NodeSchema {
        let mut schema = NodeSchema::default();
        let mut properties = HashMap::new();

        properties.insert(
            "valid".to_string(),
            PropertySchema {
                property_type: "boolean".to_string(),
                description: "Whether validation passed".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "data".to_string(),
            PropertySchema {
                property_type: "object".to_string(),
                description: "The validated data".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        properties.insert(
            "errors".to_string(),
            PropertySchema {
                property_type: "array".to_string(),
                description: "List of validation errors (if any)".to_string(),
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
            cacheable: true,
            has_side_effects: false,
            idempotent: true,
            resource_requirements: ResourceRequirements {
                memory_mb: Some(5),
                cpu_percent: Some(2.0),
                network_io: false,
                disk_io: false,
                external_dependencies: vec![],
            },
        }
    }
}

impl Default for ValidatorNode {
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
            json!({"body": {"email": "test@example.com", "age": 25}}),
        )
    }

    #[test]
    fn test_validator_node_creation() {
        let node = ValidatorNode::new();
        assert_eq!(node.node_type(), "validator");
    }

    #[tokio::test]
    async fn test_email_validation() {
        let node = ValidatorNode::new();
        let mut context = create_test_context();

        let input = NodeInput::new(
            json!({
                "input": "$trigger.body.email",
                "rules": [
                    {
                        "field": "email",
                        "type": "string",
                        "required": true,
                        "email": true
                    }
                ]
            }),
            json!({}),
        );

        let result = node.execute(&mut context, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        let valid = output.data.get("valid").unwrap().as_bool().unwrap();
        assert!(valid);
    }

    #[tokio::test]
    async fn test_validation_failure() {
        let node = ValidatorNode::new();
        let mut context = ExecutionContext::new(
            Uuid::new_v4(),
            json!({"body": {"email": "invalid-email", "age": -5}}),
        );

        let input = NodeInput::new(
            json!({
                "rules": [
                    {
                        "field": "email",
                        "type": "string",
                        "required": true,
                        "email": true
                    },
                    {
                        "field": "age",
                        "type": "number",
                        "minimum": 0
                    }
                ]
            }),
            json!({}),
        );

        let result = node.execute(&mut context, input).await;
        assert!(result.is_ok());

        let output = result.unwrap();
        let valid = output.data.get("valid").unwrap().as_bool().unwrap();
        assert!(!valid);

        let errors = output.data.get("errors").unwrap().as_array().unwrap();
        assert!(!errors.is_empty());
    }
}
