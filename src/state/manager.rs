//! State manager implementation

use crate::core::{
    execution::{ExecutionResult, ExecutionState, ExecutionStatus},
    workflow::WorkflowDefinition,
};
use crate::error::Result;
use crate::state::traits::StateManagerTrait;
use chrono::{DateTime, Utc};
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

    /// List all workflows with pagination
    pub async fn list_workflows(
        &self,
        page: u32,
        limit: u32,
        sort: Option<String>,
        order: Option<String>,
    ) -> Result<(Vec<WorkflowDefinition>, usize)> {
        let workflows = self.workflows.read().await;
        let mut workflow_list: Vec<WorkflowDefinition> = workflows.values().cloned().collect();

        let total = workflow_list.len();

        // Sort workflows
        let sort_field = sort.as_deref().unwrap_or("created_at");
        let is_desc = order.as_deref() == Some("desc");

        workflow_list.sort_by(|a, b| {
            let ordering = match sort_field {
                "name" => a.metadata.name.cmp(&b.metadata.name),
                "updated_at" => a.updated_at.cmp(&b.updated_at),
                "created_at" => a.created_at.cmp(&b.created_at),
                _ => a.created_at.cmp(&b.created_at),
            };

            if is_desc {
                ordering.reverse()
            } else {
                ordering
            }
        });

        // Apply pagination
        let start = (page * limit) as usize;
        let end = std::cmp::min(start + limit as usize, workflow_list.len());

        if start >= workflow_list.len() {
            return Ok((vec![], total));
        }

        let paginated_workflows = workflow_list[start..end].to_vec();

        Ok((paginated_workflows, total))
    }

    /// List all execution results with pagination
    pub async fn list_executions(
        &self,
        page: u32,
        limit: u32,
        sort: Option<String>,
        order: Option<String>,
        workflow_id: Option<Uuid>,
    ) -> Result<(Vec<ExecutionResult>, usize)> {
        let results = self.execution_results.read().await;
        let mut execution_list: Vec<ExecutionResult> = results.values().cloned().collect();

        // Filter by workflow_id if provided
        if let Some(wf_id) = workflow_id {
            execution_list.retain(|exec| exec.workflow_id == wf_id);
        }

        let total = execution_list.len();

        // Sort executions
        let sort_field = sort.as_deref().unwrap_or("started_at");
        let is_desc = order.as_deref() == Some("desc");

        execution_list.sort_by(|a, b| {
            let ordering = match sort_field {
                "status" => a.status.to_string().cmp(&b.status.to_string()),
                "duration_ms" => a.duration_ms.unwrap_or(0).cmp(&b.duration_ms.unwrap_or(0)),
                "completed_at" => a.completed_at.cmp(&b.completed_at),
                "started_at" => a.started_at.cmp(&b.started_at),
                _ => a.started_at.cmp(&b.started_at),
            };

            if is_desc {
                ordering.reverse()
            } else {
                ordering
            }
        });

        // Apply pagination
        let start = (page * limit) as usize;
        let end = std::cmp::min(start + limit as usize, execution_list.len());

        if start >= execution_list.len() {
            return Ok((vec![], total));
        }

        let paginated_executions = execution_list[start..end].to_vec();

        Ok((paginated_executions, total))
    }

    /// Update a workflow definition
    pub async fn update_workflow(
        &self,
        workflow_id: Uuid,
        workflow: &WorkflowDefinition,
    ) -> Result<()> {
        let mut workflows = self.workflows.write().await;

        if let std::collections::hash_map::Entry::Occupied(mut e) = workflows.entry(workflow_id) {
            e.insert(workflow.clone());
            Ok(())
        } else {
            Err(crate::error::AutomataError::Execution(
                crate::error::ExecutionError::WorkflowNotFound {
                    workflow_id: workflow_id.to_string(),
                },
            ))
        }
    }

    /// Delete a workflow definition
    pub async fn delete_workflow(&self, workflow_id: Uuid) -> Result<()> {
        let mut workflows = self.workflows.write().await;

        if workflows.remove(&workflow_id).is_some() {
            Ok(())
        } else {
            Err(crate::error::AutomataError::Execution(
                crate::error::ExecutionError::WorkflowNotFound {
                    workflow_id: workflow_id.to_string(),
                },
            ))
        }
    }

    /// Get the latest execution for a workflow
    pub async fn get_latest_execution_for_workflow(
        &self,
        workflow_id: Uuid,
    ) -> Option<chrono::DateTime<chrono::Utc>> {
        let results = self.execution_results.read().await;

        results
            .values()
            .filter(|exec| exec.workflow_id == workflow_id)
            .map(|exec| exec.started_at)
            .max()
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl StateManagerTrait for StateManager {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn load_workflow(&self, workflow_id: Uuid) -> Result<WorkflowDefinition> {
        self.load_workflow(workflow_id).await
    }

    async fn save_workflow(&self, workflow: &WorkflowDefinition) -> Result<()> {
        self.save_workflow(workflow).await
    }

    async fn init_execution(
        &self,
        execution_id: Uuid,
        workflow: &WorkflowDefinition,
    ) -> Result<()> {
        self.init_execution(execution_id, workflow).await
    }

    async fn save_execution_state(&self, execution_id: Uuid, state: &ExecutionState) -> Result<()> {
        self.save_execution_state(execution_id, state).await
    }

    async fn get_execution_status(&self, execution_id: Uuid) -> Result<ExecutionStatus> {
        self.get_execution_status(execution_id).await
    }

    async fn update_execution_status(
        &self,
        execution_id: Uuid,
        status: ExecutionStatus,
    ) -> Result<()> {
        self.update_execution_status(execution_id, status).await
    }

    async fn finalize_execution(&self, execution_id: Uuid, result: &ExecutionResult) -> Result<()> {
        self.finalize_execution(execution_id, result).await
    }

    async fn get_execution_result(&self, execution_id: Uuid) -> Result<ExecutionResult> {
        self.get_execution_result(execution_id).await
    }

    async fn list_workflows(
        &self,
        page: u32,
        limit: u32,
        sort: Option<String>,
        order: Option<String>,
    ) -> Result<(Vec<WorkflowDefinition>, usize)> {
        self.list_workflows(page, limit, sort, order).await
    }

    async fn list_executions(
        &self,
        page: u32,
        limit: u32,
        sort: Option<String>,
        order: Option<String>,
        workflow_id: Option<Uuid>,
    ) -> Result<(Vec<ExecutionResult>, usize)> {
        self.list_executions(page, limit, sort, order, workflow_id)
            .await
    }

    async fn update_workflow(
        &self,
        workflow_id: Uuid,
        workflow: &WorkflowDefinition,
    ) -> Result<()> {
        self.update_workflow(workflow_id, workflow).await
    }

    async fn delete_workflow(&self, workflow_id: Uuid) -> Result<()> {
        self.delete_workflow(workflow_id).await
    }

    async fn get_latest_execution_for_workflow(&self, workflow_id: Uuid) -> Option<DateTime<Utc>> {
        self.get_latest_execution_for_workflow(workflow_id).await
    }
}
