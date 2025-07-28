//! Schema validation for workflow definitions

use crate::error::{DslError, Result, ValidationError};
use serde_json::Value;
use std::collections::HashMap;

/// Schema validator for workflow definitions
#[derive(Debug, Clone)]
pub struct SchemaValidator {
    schema: WorkflowSchema,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
}

/// Workflow schema definition
#[derive(Debug, Clone)]
pub struct WorkflowSchema {
    pub metadata_schema: MetadataSchema,
    pub trigger_schemas: HashMap<String, TriggerSchema>,
    pub node_schemas: HashMap<String, NodeSchema>,
}

/// Metadata section schema
#[derive(Debug, Clone)]
pub struct MetadataSchema {
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
}

/// Trigger schema
#[derive(Debug, Clone)]
pub struct TriggerSchema {
    pub trigger_type: String,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
}

/// Node schema
#[derive(Debug, Clone)]
pub struct NodeSchema {
    pub node_type: String,
    pub required_fields: Vec<String>,
    pub optional_fields: Vec<String>,
    pub input_schema: Option<Value>,
    pub output_schema: Option<Value>,
}

impl SchemaValidator {
    /// Create a new schema validator
    pub fn new() -> Self {
        Self {
            schema: WorkflowSchema::default(),
        }
    }

    /// Validate a workflow definition
    pub fn validate(&self, value: &Value) -> Result<ValidationResult> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let obj = value.as_object().ok_or_else(|| DslError::InvalidSyntax {
            line: 1,
            message: "Root must be an object".to_string(),
        })?;

        // Validate metadata
        if let Some(metadata) = obj.get("@metadata") {
            self.validate_metadata(metadata, &mut errors, &mut warnings)?;
        } else {
            warnings.push("Missing @metadata section - using defaults".to_string());
        }

        // Validate triggers
        if let Some(triggers) = obj.get("triggers") {
            self.validate_triggers(triggers, &mut errors, &mut warnings)?;
        } else {
            warnings.push(
                "No triggers defined - workflow cannot be automatically executed".to_string(),
            );
        }

        // Validate nodes
        if let Some(nodes) = obj.get("nodes") {
            self.validate_nodes(nodes, &mut errors, &mut warnings)?;
        } else {
            errors.push(ValidationError::RequiredFieldMissing {
                field: "nodes".to_string(),
            });
        }

        // Validate connections
        if let Some(connections) = obj.get("connections") {
            self.validate_connections(connections, obj.get("nodes"), &mut errors, &mut warnings)?;
        }

        // Validate tests
        if let Some(tests) = obj.get("@test") {
            self.validate_tests(tests, &mut errors, &mut warnings)?;
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// Validate metadata section
    fn validate_metadata(
        &self,
        metadata: &Value,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut [String],
    ) -> Result<()> {
        let obj = metadata
            .as_object()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: "@metadata".to_string(),
                message: "Must be an object".to_string(),
            })?;

        // Check required fields
        for field in &self.schema.metadata_schema.required_fields {
            if !obj.contains_key(field) {
                errors.push(ValidationError::RequiredFieldMissing {
                    field: format!("@metadata.{field}"),
                });
            }
        }

        // Validate field types
        if let Some(name) = obj.get("name") {
            if !name.is_string() {
                errors.push(ValidationError::InvalidValue {
                    field: "@metadata.name".to_string(),
                    message: "Must be a string".to_string(),
                });
            } else if name.as_str().unwrap().is_empty() {
                errors.push(ValidationError::InvalidValue {
                    field: "@metadata.name".to_string(),
                    message: "Cannot be empty".to_string(),
                });
            }
        }

        if let Some(version) = obj.get("version") {
            if !version.is_string() {
                errors.push(ValidationError::InvalidValue {
                    field: "@metadata.version".to_string(),
                    message: "Must be a string".to_string(),
                });
            } else {
                let version_str = version.as_str().unwrap();
                if !self.is_valid_semver(version_str) {
                    errors.push(ValidationError::InvalidValue {
                        field: "@metadata.version".to_string(),
                        message: "Must be a valid semantic version (e.g., 1.0.0)".to_string(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Validate triggers section
    fn validate_triggers(
        &self,
        triggers: &Value,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut [String],
    ) -> Result<()> {
        let arr = triggers
            .as_array()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: "triggers".to_string(),
                message: "Must be an array".to_string(),
            })?;

        for (i, trigger) in arr.iter().enumerate() {
            self.validate_single_trigger(trigger, i, errors)?;
        }

        Ok(())
    }

    /// Validate a single trigger
    fn validate_single_trigger(
        &self,
        trigger: &Value,
        index: usize,
        errors: &mut Vec<ValidationError>,
    ) -> Result<()> {
        let obj = trigger
            .as_object()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: format!("triggers[{index}]"),
                message: "Must be an object".to_string(),
            })?;

        // Determine trigger type
        let trigger_types = ["webhook", "schedule", "event", "manual"];
        let mut found_type = None;

        for trigger_type in &trigger_types {
            if obj.contains_key(*trigger_type) {
                if found_type.is_some() {
                    errors.push(ValidationError::InvalidValue {
                        field: format!("triggers[{index}]"),
                        message: "Trigger can only have one type".to_string(),
                    });
                    return Ok(());
                }
                found_type = Some(*trigger_type);
            }
        }

        let trigger_type = found_type.ok_or_else(|| ValidationError::RequiredFieldMissing {
            field: format!("triggers[{index}] type"),
        })?;

        // Validate specific trigger type
        if let Some(schema) = self.schema.trigger_schemas.get(trigger_type) {
            let trigger_config = obj.get(trigger_type).unwrap();
            self.validate_trigger_config(trigger_config, schema, trigger_type, index, errors)?;
        }

        Ok(())
    }

    /// Validate trigger configuration
    fn validate_trigger_config(
        &self,
        config: &Value,
        schema: &TriggerSchema,
        trigger_type: &str,
        index: usize,
        errors: &mut Vec<ValidationError>,
    ) -> Result<()> {
        let obj = config
            .as_object()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: format!("triggers[{index}].{trigger_type}"),
                message: "Must be an object".to_string(),
            })?;

        // Check required fields
        for field in &schema.required_fields {
            if !obj.contains_key(field) {
                errors.push(ValidationError::RequiredFieldMissing {
                    field: format!("triggers[{index}].{trigger_type}.{field}"),
                });
            }
        }

        // Validate specific fields based on trigger type
        match trigger_type {
            "webhook" => {
                if let Some(method) = obj.get("method") {
                    if let Some(method_str) = method.as_str() {
                        let valid_methods = ["GET", "POST", "PUT", "DELETE", "PATCH"];
                        if !valid_methods.contains(&method_str) {
                            errors.push(ValidationError::InvalidValue {
                                field: format!("triggers[{index}].webhook.method"),
                                message: format!("Must be one of: {}", valid_methods.join(", ")),
                            });
                        }
                    }
                }
            }
            "schedule" => {
                if let Some(cron) = obj.get("cron") {
                    if let Some(cron_str) = cron.as_str() {
                        if !self.is_valid_cron(cron_str) {
                            errors.push(ValidationError::InvalidValue {
                                field: format!("triggers[{index}].schedule.cron"),
                                message: "Must be a valid cron expression".to_string(),
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Validate nodes section
    fn validate_nodes(
        &self,
        nodes: &Value,
        errors: &mut Vec<ValidationError>,
        warnings: &mut [String],
    ) -> Result<()> {
        let obj = nodes
            .as_object()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: "nodes".to_string(),
                message: "Must be an object".to_string(),
            })?;

        if obj.is_empty() {
            errors.push(ValidationError::InvalidValue {
                field: "nodes".to_string(),
                message: "Must contain at least one node".to_string(),
            });
            return Ok(());
        }

        for (node_id, node_def) in obj {
            self.validate_single_node(node_id, node_def, errors, warnings)?;
        }

        Ok(())
    }

    /// Validate a single node
    fn validate_single_node(
        &self,
        node_id: &str,
        node_def: &Value,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut [String],
    ) -> Result<()> {
        let obj = node_def
            .as_object()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: format!("nodes.{node_id}"),
                message: "Must be an object".to_string(),
            })?;

        // Check for required 'type' field
        let node_type = obj
            .get("type")
            .ok_or_else(|| ValidationError::RequiredFieldMissing {
                field: format!("nodes.{node_id}.type"),
            })?;

        let type_str = node_type
            .as_str()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: format!("nodes.{node_id}.type"),
                message: "Must be a string".to_string(),
            })?;

        // Validate against node schema if available
        if let Some(schema) = self.schema.node_schemas.get(type_str) {
            for field in &schema.required_fields {
                if !obj.contains_key(field) {
                    errors.push(ValidationError::RequiredFieldMissing {
                        field: format!("nodes.{node_id}.{field}"),
                    });
                }
            }
        }

        // Validate timeout if present
        if let Some(timeout) = obj.get("timeout") {
            if let Some(timeout_num) = timeout.as_u64() {
                if timeout_num == 0 {
                    errors.push(ValidationError::InvalidValue {
                        field: format!("nodes.{node_id}.timeout"),
                        message: "Timeout must be greater than 0".to_string(),
                    });
                }
            } else {
                errors.push(ValidationError::InvalidValue {
                    field: format!("nodes.{node_id}.timeout"),
                    message: "Must be a positive integer".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Validate connections section
    fn validate_connections(
        &self,
        connections: &Value,
        nodes: Option<&Value>,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut [String],
    ) -> Result<()> {
        let arr = connections
            .as_array()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: "connections".to_string(),
                message: "Must be an array".to_string(),
            })?;

        // Get node IDs for validation
        let node_ids: Vec<String> = if let Some(nodes_val) = nodes {
            if let Some(nodes_obj) = nodes_val.as_object() {
                nodes_obj.keys().cloned().collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        for (i, connection) in arr.iter().enumerate() {
            self.validate_single_connection(connection, i, &node_ids, errors)?;
        }

        Ok(())
    }

    /// Validate a single connection
    fn validate_single_connection(
        &self,
        connection: &Value,
        index: usize,
        node_ids: &[String],
        errors: &mut Vec<ValidationError>,
    ) -> Result<()> {
        let obj = connection
            .as_object()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: format!("connections[{index}]"),
                message: "Must be an object".to_string(),
            })?;

        // Check required fields
        let from = obj
            .get("from")
            .ok_or_else(|| ValidationError::RequiredFieldMissing {
                field: format!("connections[{index}].from"),
            })?;

        let to = obj
            .get("to")
            .ok_or_else(|| ValidationError::RequiredFieldMissing {
                field: format!("connections[{index}].to"),
            })?;

        // Validate node references
        if let Some(from_str) = from.as_str() {
            if from_str != "trigger" && !node_ids.contains(&from_str.to_string()) {
                errors.push(ValidationError::InvalidValue {
                    field: format!("connections[{index}].from"),
                    message: format!("References unknown node: {from_str}"),
                });
            }
        }

        if let Some(to_str) = to.as_str() {
            if !node_ids.contains(&to_str.to_string()) {
                errors.push(ValidationError::InvalidValue {
                    field: format!("connections[{index}].to"),
                    message: format!("References unknown node: {to_str}"),
                });
            }
        }

        Ok(())
    }

    /// Validate tests section
    fn validate_tests(
        &self,
        tests: &Value,
        errors: &mut Vec<ValidationError>,
        _warnings: &mut [String],
    ) -> Result<()> {
        let arr = tests
            .as_array()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: "@test".to_string(),
                message: "Must be an array".to_string(),
            })?;

        for (i, test) in arr.iter().enumerate() {
            self.validate_single_test(test, i, errors)?;
        }

        Ok(())
    }

    /// Validate a single test
    fn validate_single_test(
        &self,
        test: &Value,
        index: usize,
        errors: &mut Vec<ValidationError>,
    ) -> Result<()> {
        let obj = test
            .as_object()
            .ok_or_else(|| ValidationError::InvalidValue {
                field: format!("@test[{index}]"),
                message: "Must be an object".to_string(),
            })?;

        // Check required fields
        let required_fields = ["name", "input", "expect"];
        for field in &required_fields {
            if !obj.contains_key(*field) {
                errors.push(ValidationError::RequiredFieldMissing {
                    field: format!("@test[{index}].{field}"),
                });
            }
        }

        // Validate name is non-empty string
        if let Some(name) = obj.get("name") {
            if let Some(name_str) = name.as_str() {
                if name_str.is_empty() {
                    errors.push(ValidationError::InvalidValue {
                        field: format!("@test[{index}].name"),
                        message: "Cannot be empty".to_string(),
                    });
                }
            } else {
                errors.push(ValidationError::InvalidValue {
                    field: format!("@test[{index}].name"),
                    message: "Must be a string".to_string(),
                });
            }
        }

        Ok(())
    }

    /// Check if a string is a valid semantic version
    fn is_valid_semver(&self, version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }

        for part in parts {
            if part.parse::<u32>().is_err() {
                return false;
            }
        }

        true
    }

    /// Check if a string is a valid cron expression (basic validation)
    fn is_valid_cron(&self, cron: &str) -> bool {
        let parts: Vec<&str> = cron.split_whitespace().collect();
        // Basic validation: should have 5 or 6 parts
        parts.len() == 5 || parts.len() == 6
    }
}

impl Default for SchemaValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for WorkflowSchema {
    fn default() -> Self {
        let mut trigger_schemas = HashMap::new();

        // Webhook trigger schema
        trigger_schemas.insert(
            "webhook".to_string(),
            TriggerSchema {
                trigger_type: "webhook".to_string(),
                required_fields: vec!["path".to_string()],
                optional_fields: vec![
                    "method".to_string(),
                    "auth".to_string(),
                    "headers".to_string(),
                ],
            },
        );

        // Schedule trigger schema
        trigger_schemas.insert(
            "schedule".to_string(),
            TriggerSchema {
                trigger_type: "schedule".to_string(),
                required_fields: vec!["cron".to_string()],
                optional_fields: vec!["timezone".to_string()],
            },
        );

        // Event trigger schema
        trigger_schemas.insert(
            "event".to_string(),
            TriggerSchema {
                trigger_type: "event".to_string(),
                required_fields: vec!["source".to_string(), "type".to_string()],
                optional_fields: vec!["filters".to_string()],
            },
        );

        // Manual trigger schema
        trigger_schemas.insert(
            "manual".to_string(),
            TriggerSchema {
                trigger_type: "manual".to_string(),
                required_fields: vec![],
                optional_fields: vec!["parameters".to_string()],
            },
        );

        let mut node_schemas = HashMap::new();

        // HTTP node schema
        node_schemas.insert(
            "http".to_string(),
            NodeSchema {
                node_type: "http".to_string(),
                required_fields: vec!["url".to_string()],
                optional_fields: vec![
                    "method".to_string(),
                    "headers".to_string(),
                    "body".to_string(),
                    "timeout".to_string(),
                ],
                input_schema: None,
                output_schema: None,
            },
        );

        // Validator node schema
        node_schemas.insert(
            "validator".to_string(),
            NodeSchema {
                node_type: "validator".to_string(),
                required_fields: vec!["rules".to_string()],
                optional_fields: vec!["input".to_string()],
                input_schema: None,
                output_schema: None,
            },
        );

        Self {
            metadata_schema: MetadataSchema {
                required_fields: vec!["name".to_string(), "version".to_string()],
                optional_fields: vec![
                    "description".to_string(),
                    "tags".to_string(),
                    "author".to_string(),
                    "organization".to_string(),
                ],
            },
            trigger_schemas,
            node_schemas,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_workflow() {
        let validator = SchemaValidator::new();
        let workflow = json!({
            "@metadata": {
                "name": "Test Workflow",
                "version": "1.0.0"
            },
            "triggers": [{
                "webhook": {
                    "path": "/api/test",
                    "method": "POST"
                }
            }],
            "nodes": {
                "test": {
                    "type": "http",
                    "url": "https://example.com"
                }
            },
            "connections": [{
                "from": "trigger",
                "to": "test"
            }]
        });

        let result = validator.validate(&workflow).unwrap();
        assert!(result.is_valid);
    }

    #[test]
    fn test_invalid_workflow() {
        let validator = SchemaValidator::new();
        let workflow = json!({
            "@metadata": {
                "name": "",
                "version": "invalid"
            },
            "nodes": {},
            "connections": [{
                "from": "nonexistent",
                "to": "also_nonexistent"
            }]
        });

        let result = validator.validate(&workflow).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }
}
