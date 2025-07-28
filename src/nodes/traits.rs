//! Core traits and types for workflow nodes

use crate::core::execution::ExecutionContext;
use crate::error::{NodeError, Result, ValidationError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Main trait that all workflow nodes must implement
#[async_trait]
pub trait Node: Send + Sync {
    /// Get the node type identifier
    fn node_type(&self) -> &'static str;

    /// Validate the node configuration
    async fn validate_config(&self, config: &Value) -> Result<()>;

    /// Execute the node with given input and context
    async fn execute(&self, context: &mut ExecutionContext, input: NodeInput)
        -> Result<NodeOutput>;

    /// Get node description for documentation
    fn describe(&self) -> NodeDescription {
        NodeDescription {
            node_type: self.node_type().to_string(),
            description: "No description available".to_string(),
            inputs: self.input_schema(),
            outputs: self.output_schema(),
            config: self.config_schema(),
            examples: vec![],
        }
    }

    /// Get input schema for validation
    fn input_schema(&self) -> NodeSchema {
        NodeSchema::default()
    }

    /// Get output schema for validation
    fn output_schema(&self) -> NodeSchema {
        NodeSchema::default()
    }

    /// Get configuration schema for validation
    fn config_schema(&self) -> NodeSchema {
        NodeSchema::default()
    }

    /// Get supported node capabilities
    fn capabilities(&self) -> NodeCapabilities {
        NodeCapabilities::default()
    }

    /// Initialize the node (called once when registering)
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }

    /// Shutdown the node (called when unregistering)
    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Input data for node execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeInput {
    /// Node configuration from workflow definition
    pub config: Value,
    /// Input data from previous nodes or trigger
    pub data: Value,
}

/// Output data from node execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeOutput {
    /// Output data to pass to next nodes
    pub data: Value,
    /// Metadata about the execution
    pub metadata: NodeMetadata,
}

/// Metadata about node execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeMetadata {
    /// Success status
    pub success: bool,
    /// Execution duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Number of records processed (for data nodes)
    pub records_processed: Option<u64>,
    /// HTTP status code (for HTTP nodes)
    pub http_status: Option<u16>,
    /// Custom metadata
    pub custom: HashMap<String, Value>,
}

/// Schema definition for validation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeSchema {
    /// Schema type (object, array, string, etc.)
    pub schema_type: String,
    /// Required fields for object schemas
    pub required: Vec<String>,
    /// Property definitions for object schemas
    pub properties: HashMap<String, PropertySchema>,
    /// Additional schema constraints
    pub constraints: SchemaConstraints,
}

/// Property schema for object properties
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PropertySchema {
    /// Property type
    pub property_type: String,
    /// Property description
    pub description: String,
    /// Default value
    pub default: Option<Value>,
    /// Allowed values (enum)
    pub allowed_values: Option<Vec<Value>>,
    /// Minimum value (for numbers)
    pub minimum: Option<f64>,
    /// Maximum value (for numbers)
    pub maximum: Option<f64>,
    /// Pattern (for strings)
    pub pattern: Option<String>,
}

/// Schema constraints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SchemaConstraints {
    /// Minimum length (for strings/arrays)
    pub min_length: Option<usize>,
    /// Maximum length (for strings/arrays)
    pub max_length: Option<usize>,
    /// Minimum number of items (for arrays)
    pub min_items: Option<usize>,
    /// Maximum number of items (for arrays)
    pub max_items: Option<usize>,
    /// Additional properties allowed (for objects)
    pub additional_properties: bool,
}

/// Node description for documentation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeDescription {
    /// Node type identifier
    pub node_type: String,
    /// Human-readable description
    pub description: String,
    /// Input schema
    pub inputs: NodeSchema,
    /// Output schema
    pub outputs: NodeSchema,
    /// Configuration schema
    pub config: NodeSchema,
    /// Usage examples
    pub examples: Vec<NodeExample>,
}

/// Example of node usage
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeExample {
    /// Example name
    pub name: String,
    /// Example description
    pub description: String,
    /// Example configuration
    pub config: Value,
    /// Example input
    pub input: Value,
    /// Expected output
    pub output: Value,
}

/// Node capabilities
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeCapabilities {
    /// Whether the node supports streaming data
    pub supports_streaming: bool,
    /// Whether the node supports batch processing
    pub supports_batch: bool,
    /// Whether the node can be cached
    pub cacheable: bool,
    /// Whether the node has side effects
    pub has_side_effects: bool,
    /// Whether the node is idempotent
    pub idempotent: bool,
    /// Resource requirements
    pub resource_requirements: ResourceRequirements,
}

/// Resource requirements for node execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct ResourceRequirements {
    /// Estimated memory usage in MB
    pub memory_mb: Option<u64>,
    /// Estimated CPU usage percentage
    pub cpu_percent: Option<f64>,
    /// Network I/O requirement
    pub network_io: bool,
    /// Disk I/O requirement
    pub disk_io: bool,
    /// External dependencies
    pub external_dependencies: Vec<String>,
}

/// Node category for organization
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq, Hash)]
pub enum NodeCategory {
    /// Data transformation nodes
    Transform,
    /// HTTP/API integration nodes
    Http,
    /// Database operation nodes
    Database,
    /// Control flow nodes
    Control,
    /// Validation nodes
    Validation,
    /// Communication nodes (email, webhooks, etc.)
    Communication,
    /// File/Storage operation nodes
    Storage,
    /// Custom/Extension nodes
    Custom,
}

impl NodeInput {
    /// Create a new node input
    pub fn new(config: Value, data: Value) -> Self {
        Self { config, data }
    }

    /// Get a configuration value by key
    pub fn get_config<T>(&self, key: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let value = self.config.get(key).ok_or_else(|| {
            crate::error::AutomataError::Node(NodeError::MissingInput {
                input_name: key.to_string(),
            })
        })?;

        serde_json::from_value(value.clone()).map_err(|_e| {
            crate::error::AutomataError::Node(NodeError::InvalidInputType {
                input_name: key.to_string(),
                expected: std::any::type_name::<T>().to_string(),
                actual: format!("{value:?}"),
            })
        })
    }

    /// Get optional configuration value
    pub fn get_config_optional<T>(&self, key: &str) -> Result<Option<T>>
    where
        T: serde::de::DeserializeOwned,
    {
        match self.config.get(key) {
            Some(value) => Ok(Some(serde_json::from_value(value.clone())?)),
            None => Ok(None),
        }
    }

    /// Get configuration with default value
    pub fn get_config_or<T>(&self, key: &str, default: T) -> T
    where
        T: serde::de::DeserializeOwned + Clone,
    {
        self.get_config_optional(key)
            .unwrap_or(None)
            .unwrap_or(default)
    }
}

impl NodeOutput {
    /// Create a new successful output
    pub fn success(data: Value) -> Self {
        Self {
            data,
            metadata: NodeMetadata::success(),
        }
    }

    /// Create a new output with custom metadata
    pub fn with_metadata(data: Value, metadata: NodeMetadata) -> Self {
        Self { data, metadata }
    }

    /// Create an empty success output
    pub fn empty() -> Self {
        Self::success(Value::Null)
    }
}

impl NodeMetadata {
    /// Create successful metadata
    pub fn success() -> Self {
        Self {
            success: true,
            duration_ms: None,
            records_processed: None,
            http_status: None,
            custom: HashMap::new(),
        }
    }

    /// Create failed metadata
    pub fn failed() -> Self {
        Self {
            success: false,
            duration_ms: None,
            records_processed: None,
            http_status: None,
            custom: HashMap::new(),
        }
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Set HTTP status
    pub fn with_http_status(mut self, status: u16) -> Self {
        self.http_status = Some(status);
        self
    }

    /// Set records processed
    pub fn with_records_processed(mut self, count: u64) -> Self {
        self.records_processed = Some(count);
        self
    }

    /// Add custom metadata
    pub fn with_custom(mut self, key: String, value: Value) -> Self {
        self.custom.insert(key, value);
        self
    }
}

impl Default for NodeSchema {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            required: vec![],
            properties: HashMap::new(),
            constraints: SchemaConstraints::default(),
        }
    }
}

impl Default for SchemaConstraints {
    fn default() -> Self {
        Self {
            min_length: None,
            max_length: None,
            min_items: None,
            max_items: None,
            additional_properties: true,
        }
    }
}

impl Default for NodeCapabilities {
    fn default() -> Self {
        Self {
            supports_streaming: false,
            supports_batch: true,
            cacheable: false,
            has_side_effects: true,
            idempotent: false,
            resource_requirements: ResourceRequirements::default(),
        }
    }
}

/// Helper trait for validating node configurations
pub trait NodeValidator {
    /// Validate configuration against schema
    fn validate_config(config: &Value, schema: &NodeSchema) -> Result<()> {
        if schema.schema_type == "object" {
            Self::validate_object(config, schema)
        } else {
            Ok(()) // Basic validation for now
        }
    }

    /// Validate object configuration
    fn validate_object(config: &Value, schema: &NodeSchema) -> Result<()> {
        let obj = config.as_object().ok_or_else(|| {
            crate::error::AutomataError::Validation(ValidationError::InvalidValue {
                field: "config".to_string(),
                message: "Must be an object".to_string(),
            })
        })?;

        // Check required fields
        for required_field in &schema.required {
            if !obj.contains_key(required_field) {
                return Err(crate::error::AutomataError::Validation(
                    ValidationError::RequiredFieldMissing {
                        field: required_field.clone(),
                    },
                ));
            }
        }

        // Validate properties
        for (field_name, field_value) in obj {
            if let Some(property_schema) = schema.properties.get(field_name) {
                Self::validate_property(field_name, field_value, property_schema)?;
            } else if !schema.constraints.additional_properties {
                return Err(crate::error::AutomataError::Validation(
                    ValidationError::InvalidValue {
                        field: field_name.clone(),
                        message: "Additional property not allowed".to_string(),
                    },
                ));
            }
        }

        Ok(())
    }

    /// Validate a single property
    fn validate_property(field_name: &str, value: &Value, schema: &PropertySchema) -> Result<()> {
        // Type validation
        match schema.property_type.as_str() {
            "string" => {
                if !value.is_string() {
                    return Err(crate::error::AutomataError::Validation(
                        ValidationError::InvalidValue {
                            field: field_name.to_string(),
                            message: "Must be a string".to_string(),
                        },
                    ));
                }

                // Pattern validation
                if let (Some(pattern), Some(string_value)) = (&schema.pattern, value.as_str()) {
                    let regex = regex::Regex::new(pattern).map_err(|_| {
                        crate::error::AutomataError::Validation(ValidationError::InvalidValue {
                            field: field_name.to_string(),
                            message: "Invalid pattern".to_string(),
                        })
                    })?;

                    if !regex.is_match(string_value) {
                        return Err(crate::error::AutomataError::Validation(
                            ValidationError::InvalidValue {
                                field: field_name.to_string(),
                                message: format!("Does not match pattern: {pattern}"),
                            },
                        ));
                    }
                }
            }
            "number" => {
                if !value.is_number() {
                    return Err(crate::error::AutomataError::Validation(
                        ValidationError::InvalidValue {
                            field: field_name.to_string(),
                            message: "Must be a number".to_string(),
                        },
                    ));
                }

                if let Some(num) = value.as_f64() {
                    // Range validation
                    if let Some(min) = schema.minimum {
                        if num < min {
                            return Err(crate::error::AutomataError::Validation(
                                ValidationError::ValueOutOfRange {
                                    field: field_name.to_string(),
                                    value: num.to_string(),
                                    min: min.to_string(),
                                    max: schema
                                        .maximum
                                        .map(|m| m.to_string())
                                        .unwrap_or("∞".to_string()),
                                },
                            ));
                        }
                    }

                    if let Some(max) = schema.maximum {
                        if num > max {
                            return Err(crate::error::AutomataError::Validation(
                                ValidationError::ValueOutOfRange {
                                    field: field_name.to_string(),
                                    value: num.to_string(),
                                    min: schema
                                        .minimum
                                        .map(|m| m.to_string())
                                        .unwrap_or("-∞".to_string()),
                                    max: max.to_string(),
                                },
                            ));
                        }
                    }
                }
            }
            "boolean" => {
                if !value.is_boolean() {
                    return Err(crate::error::AutomataError::Validation(
                        ValidationError::InvalidValue {
                            field: field_name.to_string(),
                            message: "Must be a boolean".to_string(),
                        },
                    ));
                }
            }
            _ => {} // Skip validation for unknown types
        }

        // Enum validation
        if let Some(allowed_values) = &schema.allowed_values {
            if !allowed_values.contains(value) {
                return Err(crate::error::AutomataError::Validation(
                    ValidationError::InvalidValue {
                        field: field_name.to_string(),
                        message: format!("Must be one of: {allowed_values:?}"),
                    },
                ));
            }
        }

        Ok(())
    }
}

/// Base implementation of NodeValidator
pub struct BaseNodeValidator;
impl NodeValidator for BaseNodeValidator {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_node_input() {
        let config = json!({"url": "http://example.com", "timeout": 30});
        let data = json!({"user": "test"});
        let input = NodeInput::new(config, data);

        let url: String = input.get_config("url").unwrap();
        assert_eq!(url, "http://example.com");

        let timeout: u64 = input.get_config("timeout").unwrap();
        assert_eq!(timeout, 30);

        let missing: Option<String> = input.get_config_optional("missing").unwrap();
        assert!(missing.is_none());

        let default_value: u64 = input.get_config_or("missing", 60);
        assert_eq!(default_value, 60);
    }

    #[test]
    fn test_node_output() {
        let data = json!({"result": "success"});
        let output = NodeOutput::success(data.clone());

        assert_eq!(output.data, data);
        assert!(output.metadata.success);
    }

    #[test]
    fn test_schema_validation() {
        let mut schema = NodeSchema::default();
        schema.required.push("url".to_string());
        schema.properties.insert(
            "url".to_string(),
            PropertySchema {
                property_type: "string".to_string(),
                description: "URL to request".to_string(),
                default: None,
                allowed_values: None,
                minimum: None,
                maximum: None,
                pattern: None,
            },
        );

        let valid_config = json!({"url": "http://example.com"});
        assert!(BaseNodeValidator::validate_config(&valid_config, &schema).is_ok());

        let invalid_config = json!({"timeout": 30});
        assert!(BaseNodeValidator::validate_config(&invalid_config, &schema).is_err());
    }
}
