//! State manager implementation

use crate::core::{
    execution::{ExecutionResult, ExecutionState, ExecutionStatus},
    workflow::WorkflowDefinition,
};
use crate::error::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// State manager for persisting workflow and execution state
#[derive(Clone)]
pub struct StateManager {
    // For now, we'll use in-memory storage
    // In production, this would use PostgreSQL and Redis
    workflows: Arc<RwLock<HashMap<Uuid, WorkflowDefinition>>>,
    executions: Arc<RwLock<HashMap<Uuid, ExecutionState>>>,
    execution_results: Arc<RwLock<HashMap<Uuid, ExecutionResult>>>,
}

impl StateManager {
    /// Create a new state manager
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            execution_results: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a mock state manager for testing
    pub fn new_mock() -> Self {
        Self::new()
    }

    /// Load a workflow definition
    pub async fn load_workflow(&self, workflow_id: Uuid) -> Result<WorkflowDefinition> {
        let workflows = self.workflows.read().await;
        workflows.get(&workflow_id).cloned().ok_or_else(|| {
            crate::error::AutomataError::Execution(crate::error::ExecutionError::WorkflowNotFound {
                workflow_id: workflow_id.to_string(),
            })
        })
    }

    /// Save a workflow definition
    pub async fn save_workflow(&self, workflow: &WorkflowDefinition) -> Result<()> {
        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow.id, workflow.clone());
        Ok(())
    }

    /// Initialize execution state
    pub async fn init_execution(
        &self,
        execution_id: Uuid,
        workflow: &WorkflowDefinition,
    ) -> Result<()> {
        let state = ExecutionState {
            execution_id,
            workflow_id: workflow.id,
            status: ExecutionStatus::Running,
            trigger_data: serde_json::Value::Null,
            node_outputs: HashMap::new(),
            global_variables: HashMap::new(),
            current_stage: 0,
            completed_nodes: Vec::new(),
            failed_nodes: Vec::new(),
            started_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let mut executions = self.executions.write().await;
        executions.insert(execution_id, state);
        Ok(())
    }

    /// Save execution state
    pub async fn save_execution_state(
        &self,
        execution_id: Uuid,
        state: &ExecutionState,
    ) -> Result<()> {
        let mut executions = self.executions.write().await;
        executions.insert(execution_id, state.clone());
        Ok(())
    }

    /// Get execution status
    pub async fn get_execution_status(&self, execution_id: Uuid) -> Result<ExecutionStatus> {
        let executions = self.executions.read().await;
        executions
            .get(&execution_id)
            .map(|state| state.status.clone())
            .ok_or_else(|| {
                crate::error::AutomataError::Execution(
                    crate::error::ExecutionError::ExecutionNotFound {
                        execution_id: execution_id.to_string(),
                    },
                )
            })
    }

    /// Update execution status
    pub async fn update_execution_status(
        &self,
        execution_id: Uuid,
        status: ExecutionStatus,
    ) -> Result<()> {
        let mut executions = self.executions.write().await;
        if let Some(state) = executions.get_mut(&execution_id) {
            state.status = status;
            state.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    /// Finalize execution with result
    pub async fn finalize_execution(
        &self,
        execution_id: Uuid,
        result: &ExecutionResult,
    ) -> Result<()> {
        let mut results = self.execution_results.write().await;
        results.insert(execution_id, result.clone());
        Ok(())
    }

    /// Get execution result
    pub async fn get_execution_result(&self, execution_id: Uuid) -> Result<ExecutionResult> {
        let results = self.execution_results.read().await;
        results.get(&execution_id).cloned().ok_or_else(|| {
            crate::error::AutomataError::Execution(
                crate::error::ExecutionError::ExecutionNotFound {
                    execution_id: execution_id.to_string(),
                },
            )
        })
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}
