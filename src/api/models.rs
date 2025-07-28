//! API request/response models

use crate::core::{
    engine::ExecutionEngine,
    execution::ExecutionStatus,
    workflow::{WorkflowConnection, WorkflowMetadata, WorkflowNode, WorkflowTest, WorkflowTrigger},
};
use crate::nodes::{NodeDescription, NodeRegistry};
use crate::state::StateManager;
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
    pub state_manager: Arc<StateManager>,
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
}

/// Response for listing available nodes
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListNodesResponse {
    pub nodes: Vec<String>,
    pub descriptions: Vec<NodeDescription>,
    pub total: usize,
}
