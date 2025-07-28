//! State manager trait definitions

use crate::core::{
    execution::{ExecutionResult, ExecutionState, ExecutionStatus},
    workflow::WorkflowDefinition,
};
use crate::error::Result;
use chrono::{DateTime, Utc};
use std::any::Any;
use uuid::Uuid;

/// Trait for state management implementations
#[async_trait::async_trait]
pub trait StateManagerTrait: Send + Sync {
    /// Downcast to concrete type for accessing specific methods
    fn as_any(&self) -> &dyn Any;
    /// Load a workflow definition
    async fn load_workflow(&self, workflow_id: Uuid) -> Result<WorkflowDefinition>;

    /// Save a workflow definition
    async fn save_workflow(&self, workflow: &WorkflowDefinition) -> Result<()>;

    /// Initialize execution state
    async fn init_execution(&self, execution_id: Uuid, workflow: &WorkflowDefinition)
        -> Result<()>;

    /// Save execution state
    async fn save_execution_state(&self, execution_id: Uuid, state: &ExecutionState) -> Result<()>;

    /// Get execution status
    async fn get_execution_status(&self, execution_id: Uuid) -> Result<ExecutionStatus>;

    /// Update execution status
    async fn update_execution_status(
        &self,
        execution_id: Uuid,
        status: ExecutionStatus,
    ) -> Result<()>;

    /// Finalize execution with result
    async fn finalize_execution(&self, execution_id: Uuid, result: &ExecutionResult) -> Result<()>;

    /// Get execution result
    async fn get_execution_result(&self, execution_id: Uuid) -> Result<ExecutionResult>;

    /// List all workflows with pagination
    async fn list_workflows(
        &self,
        page: u32,
        limit: u32,
        sort: Option<String>,
        order: Option<String>,
    ) -> Result<(Vec<WorkflowDefinition>, usize)>;

    /// List all execution results with pagination
    async fn list_executions(
        &self,
        page: u32,
        limit: u32,
        sort: Option<String>,
        order: Option<String>,
        workflow_id: Option<Uuid>,
    ) -> Result<(Vec<ExecutionResult>, usize)>;

    /// Update a workflow definition
    async fn update_workflow(&self, workflow_id: Uuid, workflow: &WorkflowDefinition)
        -> Result<()>;

    /// Delete a workflow definition
    async fn delete_workflow(&self, workflow_id: Uuid) -> Result<()>;

    /// Get the latest execution for a workflow
    async fn get_latest_execution_for_workflow(&self, workflow_id: Uuid) -> Option<DateTime<Utc>>;
}
