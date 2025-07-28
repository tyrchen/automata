//! Database models for state persistence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Workflow record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRecord {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub definition: Value,
    pub status: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Execution record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub status: i32,
    pub trigger_data: Value,
    pub outputs: Option<Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

/// Node execution record for database storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionRecord {
    pub id: Uuid,
    pub execution_id: Uuid,
    pub node_id: String,
    pub node_type: String,
    pub status: i32,
    pub input: Value,
    pub output: Option<Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub retry_count: i32,
}

/// Execution state record for caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStateRecord {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub status: i32,
    pub trigger_data: Value,
    pub node_outputs: Value,
    pub global_variables: Value,
    pub current_stage: i32,
    pub completed_nodes: Vec<String>,
    pub failed_nodes: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
