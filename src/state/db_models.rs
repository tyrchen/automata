//! Database model structures for SQLx queries

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow)]
pub struct WorkflowRow {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub definition: String,
    pub version: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub tags: Option<Vec<String>>,
    pub metadata: Value,
}

#[derive(Debug, FromRow)]
pub struct ExecutionRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub status: String,
    pub trigger_data: Value,
    pub context: Value,
    pub outputs: Value,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, FromRow)]
pub struct ExecutionStatusRow {
    pub status: String,
    pub error: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct CountRow {
    pub count: i64,
}

#[derive(Debug, FromRow)]
pub struct MaxDateRow {
    pub latest_execution: Option<DateTime<Utc>>,
}
