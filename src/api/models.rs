//! API request/response models

use crate::core::{
    engine::ExecutionEngine,
    execution::{ExecutionStatus, NodeExecutionResult},
    workflow::{WorkflowConnection, WorkflowMetadata, WorkflowNode, WorkflowTest, WorkflowTrigger},
};
use crate::nodes::{NodeDescription, NodeRegistry};
use crate::state::StateManagerTrait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

/// Shared application state
#[derive(Clone)]
pub struct SharedState {
    pub execution_engine: Arc<ExecutionEngine>,
    pub node_registry: Arc<NodeRegistry>,
    pub state_manager: Arc<dyn StateManagerTrait>,
}

/// Request to create a new workflow
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub definition: String, // YAML workflow definition
}

/// Response for workflow creation
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateWorkflowResponse {
    pub id: Uuid,
    pub name: String,
    pub status: String,
}

/// Response for getting a workflow
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetWorkflowResponse {
    pub id: Uuid,
    pub metadata: WorkflowMetadata,
    pub triggers: Vec<WorkflowTrigger>,
    pub nodes: HashMap<String, WorkflowNode>,
    pub connections: Vec<WorkflowConnection>,
    pub tests: Option<Vec<WorkflowTest>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub definition: String, // Raw YAML definition for the DSL editor
}

/// Request to execute a workflow
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecuteWorkflowRequest {
    pub trigger_data: Value,
}

/// Response for workflow execution
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecuteWorkflowResponse {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub status: ExecutionStatus,
    pub outputs: Option<HashMap<String, Value>>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: u64,
}

/// Response for getting execution status
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetExecutionResponse {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub status: ExecutionStatus,
    pub outputs: Option<HashMap<String, Value>>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: u64,
    pub node_executions: Vec<NodeExecutionResult>,
}

/// Response for listing available nodes
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListNodesResponse {
    pub nodes: Vec<String>,
    pub descriptions: Vec<NodeDescription>,
    pub total: usize,
}

/// Query parameters for listing workflows
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListWorkflowsQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
    pub order: Option<String>, // "asc" or "desc"
}

/// Query parameters for listing executions
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListExecutionsQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub sort: Option<String>,
    pub order: Option<String>, // "asc" or "desc"
    pub workflow_id: Option<Uuid>,
}

/// List item for workflows
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WorkflowListItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub status: String, // "active", "inactive", "error"
    pub last_execution: Option<DateTime<Utc>>,
    pub node_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// List item for executions
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ExecutionListItem {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub workflow_name: String,
    pub status: ExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: u64,
    pub error: Option<String>,
}

/// Response for listing workflows
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListWorkflowsResponse {
    pub workflows: Vec<WorkflowListItem>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Response for listing executions
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListExecutionsResponse {
    pub executions: Vec<ExecutionListItem>,
    pub total: usize,
    pub page: u32,
    pub limit: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Request to update a workflow
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub definition: Option<String>, // YAML workflow definition
}

/// Response for updating a workflow
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateWorkflowResponse {
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub updated_at: DateTime<Utc>,
}

/// Response for deleting a workflow
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DeleteWorkflowResponse {
    pub id: Uuid,
    pub status: String,
}

/// Response for cancelling an execution
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CancelExecutionResponse {
    pub execution_id: Uuid,
    pub status: ExecutionStatus,
    pub cancelled_at: DateTime<Utc>,
}

/// Log entry for execution
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String, // "info", "warn", "error", "debug"
    pub message: String,
    pub node_id: Option<String>,
}

/// Response for execution logs
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetExecutionLogsResponse {
    pub execution_id: Uuid,
    pub logs: Vec<LogEntry>,
}

/// Response for re-running an execution
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RerunExecutionResponse {
    pub new_execution_id: Uuid,
    pub original_execution_id: Uuid,
    pub workflow_id: Uuid,
    pub status: String,
}
