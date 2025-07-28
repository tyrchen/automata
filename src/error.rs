//! Error handling for the Automata workflow engine

use thiserror::Error;

/// Main error type for the Automata workflow engine
#[derive(Error, Debug)]
pub enum AutomataError {
    /// DSL parsing errors
    #[error("DSL parse error: {0}")]
    DslParse(#[from] DslError),

    /// Execution errors
    #[error("Execution error: {0}")]
    Execution(#[from] ExecutionError),

    /// Node errors
    #[error("Node error: {0}")]
    Node(#[from] NodeError),

    /// Database errors
    #[error("Database error: {0}")]
    Database(String),

    /// Redis errors
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),

    /// HTTP errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// YAML errors
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    /// Parsing errors
    #[error("Parsing error: {0}")]
    Parsing(String),

    /// Authentication errors
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Timeout errors
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Resource errors
    #[error("Resource error: {0}")]
    Resource(String),

    /// Internal errors
    #[error("Internal error: {0}")]
    Internal(String),
}

/// DSL parsing errors
#[derive(Error, Debug)]
pub enum DslError {
    #[error("Invalid syntax at line {line}: {message}")]
    InvalidSyntax { line: usize, message: String },

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid field type for {field}: expected {expected}, got {actual}")]
    InvalidFieldType {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Circular dependency detected: {nodes:?}")]
    CircularDependency { nodes: Vec<String> },

    #[error("Unknown node type: {node_type}")]
    UnknownNodeType { node_type: String },

    #[error("Invalid expression: {expression}")]
    InvalidExpression { expression: String },

    #[error("Parse error: {0}")]
    Parse(#[from] pest::error::Error<crate::dsl::parser::Rule>),
}

/// Execution errors
#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error("Node execution failed: {node_id}")]
    NodeExecutionFailed { node_id: String },

    #[error("Workflow not found: {workflow_id}")]
    WorkflowNotFound { workflow_id: String },

    #[error("Execution not found: {execution_id}")]
    ExecutionNotFound { execution_id: String },

    #[error("Node timeout: {node_id} (after {timeout_ms}ms)")]
    NodeTimeout { node_id: String, timeout_ms: u64 },

    #[error("Execution timeout: {execution_id} (after {timeout_ms}ms)")]
    ExecutionTimeout {
        execution_id: String,
        timeout_ms: u64,
    },

    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },

    #[error("Invalid state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },

    #[error("Dependency not met: {dependency}")]
    DependencyNotMet { dependency: String },

    #[error("Condition evaluation failed: {condition}")]
    ConditionEvaluationFailed { condition: String },
}

/// Node-specific errors
#[derive(Error, Debug)]
pub enum NodeError {
    #[error("Node not found: {node_type}")]
    NotFound { node_type: String },

    #[error("Invalid configuration for node {node_type}: {message}")]
    InvalidConfig { node_type: String, message: String },

    #[error("Missing required input: {input_name}")]
    MissingInput { input_name: String },

    #[error("Invalid input type for {input_name}: expected {expected}, got {actual}")]
    InvalidInputType {
        input_name: String,
        expected: String,
        actual: String,
    },

    #[error("Node execution failed: {message}")]
    ExecutionFailed { message: String },

    #[error("External service error: {service} - {message}")]
    ExternalServiceError { service: String, message: String },
}

/// Authentication and authorization errors
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid token")]
    InvalidToken,

    #[error("Insufficient permissions")]
    InsufficientPermissions,

    #[error("User not found: {user_id}")]
    UserNotFound { user_id: String },

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),
}

/// Validation errors
#[derive(Error, Debug, Clone)]
pub enum ValidationError {
    #[error("Required field missing: {field}")]
    RequiredFieldMissing { field: String },

    #[error("Invalid value for field {field}: {message}")]
    InvalidValue { field: String, message: String },

    #[error("Value out of range for field {field}: {value} (expected {min}-{max})")]
    ValueOutOfRange {
        field: String,
        value: String,
        min: String,
        max: String,
    },

    #[error("Schema validation failed: {0}")]
    Schema(String),

    #[error("Validator error: {0}")]
    Validator(#[from] validator::ValidationErrors),
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, AutomataError>;

impl AutomataError {
    /// Check if the error is transient and might succeed on retry
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            AutomataError::Database(_)
                | AutomataError::Redis(_)
                | AutomataError::Http(_)
                | AutomataError::Timeout(_)
                | AutomataError::Resource(_)
        )
    }

    /// Get error code for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            AutomataError::DslParse(_) => "DSL_PARSE_ERROR",
            AutomataError::Execution(_) => "EXECUTION_ERROR",
            AutomataError::Node(_) => "NODE_ERROR",
            AutomataError::Database(_) => "DATABASE_ERROR",
            AutomataError::Redis(_) => "REDIS_ERROR",
            AutomataError::Http(_) => "HTTP_ERROR",
            AutomataError::Io(_) => "IO_ERROR",
            AutomataError::Serialization(_) => "SERIALIZATION_ERROR",
            AutomataError::Yaml(_) => "YAML_ERROR",
            AutomataError::Parsing(_) => "PARSING_ERROR",
            AutomataError::Auth(_) => "AUTH_ERROR",
            AutomataError::Validation(_) => "VALIDATION_ERROR",
            AutomataError::Config(_) => "CONFIG_ERROR",
            AutomataError::Timeout(_) => "TIMEOUT_ERROR",
            AutomataError::Resource(_) => "RESOURCE_ERROR",
            AutomataError::Internal(_) => "INTERNAL_ERROR",
        }
    }
}
