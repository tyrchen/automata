//! PostgreSQL-backed state manager implementation

use crate::core::{
    execution::{ExecutionResult, ExecutionState, ExecutionStatus},
    workflow::WorkflowDefinition,
};
use crate::error::{AutomataError, ExecutionError, Result};
use crate::state::db_models::{
    CountRow, ExecutionRow, ExecutionStatusRow, MaxDateRow, WorkflowRow,
};
use crate::state::traits::StateManagerTrait;
use chrono::{DateTime, Utc};
use sqlx::{postgres::PgPoolOptions, PgPool};
use uuid::Uuid;

/// PostgreSQL-backed state manager
#[derive(Clone)]
pub struct PostgresStateManager {
    pool: PgPool,
}

impl PostgresStateManager {
    /// Create a new PostgreSQL state manager
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(database_url)
            .await
            .map_err(|e| AutomataError::Database(format!("Failed to connect to database: {e}")))?;

        Ok(Self { pool })
    }

    /// Create from existing pool
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Load a workflow definition with raw YAML
    pub async fn load_workflow_with_definition(
        &self,
        workflow_id: Uuid,
    ) -> Result<(WorkflowDefinition, String)> {
        let row = sqlx::query_as::<_, WorkflowRow>(
            r#"
            SELECT id, name, description, definition, version, status, created_at, updated_at, tags, metadata
            FROM workflows
            WHERE id = $1 AND status = 'active'
            "#
        )
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to load workflow: {e}")))?;

        match row {
            Some(row) => {
                // Parse the YAML definition using the DSL parser
                let parser = crate::dsl::DslParser::new();
                let parsed_result = parser.parse(&row.definition).map_err(|e| {
                    AutomataError::Parsing(format!("Failed to parse workflow definition: {e}"))
                })?;

                // Return both the parsed workflow definition and raw YAML
                Ok((parsed_result.definition, row.definition))
            }
            None => Err(AutomataError::Execution(ExecutionError::WorkflowNotFound {
                workflow_id: workflow_id.to_string(),
            })),
        }
    }

    /// Load a workflow definition (for backward compatibility)
    pub async fn load_workflow(&self, workflow_id: Uuid) -> Result<WorkflowDefinition> {
        let (workflow, _) = self.load_workflow_with_definition(workflow_id).await?;
        Ok(workflow)
    }

    /// Save a workflow definition
    pub async fn save_workflow(&self, workflow: &WorkflowDefinition) -> Result<()> {
        // Convert workflow to YAML
        let definition_yaml = serde_yaml::to_string(&workflow)
            .map_err(|e| AutomataError::Parsing(format!("Failed to serialize workflow: {e}")))?;

        sqlx::query(
            r#"
            INSERT INTO workflows (id, name, description, definition, version, status, tags, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                definition = EXCLUDED.definition,
                version = EXCLUDED.version,
                status = EXCLUDED.status,
                tags = EXCLUDED.tags,
                metadata = EXCLUDED.metadata,
                updated_at = NOW()
            "#
        )
        .bind(workflow.id)
        .bind(&workflow.metadata.name)
        .bind(&workflow.metadata.description)
        .bind(definition_yaml)
        .bind(&workflow.metadata.version)
        .bind("active")
        .bind(&workflow.metadata.tags)
        .bind(serde_json::json!({}))
        .execute(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to save workflow: {e}")))?;

        Ok(())
    }

    /// Initialize execution state
    pub async fn init_execution(
        &self,
        execution_id: Uuid,
        workflow: &WorkflowDefinition,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO executions (id, workflow_id, status, trigger_data, context, outputs)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(execution_id)
        .bind(workflow.id)
        .bind("pending")
        .bind(serde_json::json!({}))
        .bind(serde_json::json!({}))
        .bind(serde_json::json!({}))
        .execute(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to initialize execution: {e}")))?;

        Ok(())
    }

    /// Save execution state
    pub async fn save_execution_state(
        &self,
        execution_id: Uuid,
        state: &ExecutionState,
    ) -> Result<()> {
        let status = match &state.status {
            ExecutionStatus::Pending => "pending",
            ExecutionStatus::Running => "running",
            ExecutionStatus::Completed => "completed",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Cancelled => "cancelled",
            ExecutionStatus::Timeout => "timeout",
        };

        let error_msg = if state.status == ExecutionStatus::Failed {
            state
                .failed_nodes
                .first()
                .map(|node| format!("Failed at node: {node}"))
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE executions
            SET status = $2,
                context = $3,
                outputs = $4,
                error = $5
            WHERE id = $1
            "#,
        )
        .bind(execution_id)
        .bind(status)
        .bind(serde_json::to_value(&state.global_variables).unwrap_or_default())
        .bind(serde_json::to_value(&state.node_outputs).unwrap_or_default())
        .bind(error_msg)
        .execute(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to save execution state: {e}")))?;

        Ok(())
    }

    /// Get execution status
    pub async fn get_execution_status(&self, execution_id: Uuid) -> Result<ExecutionStatus> {
        let row = sqlx::query_as::<_, ExecutionStatusRow>(
            r#"
            SELECT status, error
            FROM executions
            WHERE id = $1
            "#,
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to get execution status: {e}")))?;

        match row {
            Some(row) => Ok(match row.status.as_str() {
                "pending" => ExecutionStatus::Pending,
                "running" => ExecutionStatus::Running,
                "completed" => ExecutionStatus::Completed,
                "failed" => ExecutionStatus::Failed,
                "cancelled" => ExecutionStatus::Cancelled,
                "timeout" => ExecutionStatus::Timeout,
                _ => ExecutionStatus::Pending,
            }),
            None => Err(AutomataError::Execution(
                ExecutionError::ExecutionNotFound {
                    execution_id: execution_id.to_string(),
                },
            )),
        }
    }

    /// Update execution status
    pub async fn update_execution_status(
        &self,
        execution_id: Uuid,
        status: ExecutionStatus,
    ) -> Result<()> {
        let status_str = match &status {
            ExecutionStatus::Pending => "pending",
            ExecutionStatus::Running => "running",
            ExecutionStatus::Completed => "completed",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Cancelled => "cancelled",
            ExecutionStatus::Timeout => "timeout",
        };

        let error_msg: Option<String> = None; // Error details should be passed separately if needed

        let completed_at = match &status {
            ExecutionStatus::Completed
            | ExecutionStatus::Failed
            | ExecutionStatus::Cancelled
            | ExecutionStatus::Timeout => Some(Utc::now()),
            _ => None,
        };

        sqlx::query(
            r#"
            UPDATE executions
            SET status = $2,
                error = $3,
                completed_at = $4
            WHERE id = $1
            "#,
        )
        .bind(execution_id)
        .bind(status_str)
        .bind(error_msg)
        .bind(completed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to update execution status: {e}")))?;

        Ok(())
    }

    /// Finalize execution with result
    pub async fn finalize_execution(
        &self,
        execution_id: Uuid,
        result: &ExecutionResult,
    ) -> Result<()> {
        let status_str = match &result.status {
            ExecutionStatus::Pending => "pending",
            ExecutionStatus::Running => "running",
            ExecutionStatus::Completed => "completed",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::Cancelled => "cancelled",
            ExecutionStatus::Timeout => "timeout",
        };

        let error_msg = result.error.clone();

        sqlx::query(
            r#"
            UPDATE executions
            SET status = $2,
                outputs = $3,
                error = $4,
                completed_at = $5
            WHERE id = $1
            "#,
        )
        .bind(execution_id)
        .bind(status_str)
        .bind(serde_json::to_value(&result.outputs).unwrap_or_default())
        .bind(error_msg)
        .bind(result.completed_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to finalize execution: {e}")))?;

        Ok(())
    }

    /// Get execution result
    pub async fn get_execution_result(&self, execution_id: Uuid) -> Result<ExecutionResult> {
        let row = sqlx::query_as::<_, ExecutionRow>(
            r#"
            SELECT id, workflow_id, status, trigger_data, context, outputs, error, started_at, completed_at, duration_ms
            FROM executions
            WHERE id = $1
            "#
        )
        .bind(execution_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to get execution result: {e}")))?;

        match row {
            Some(row) => {
                let status = match row.status.as_str() {
                    "pending" => ExecutionStatus::Pending,
                    "running" => ExecutionStatus::Running,
                    "completed" => ExecutionStatus::Completed,
                    "failed" => ExecutionStatus::Failed,
                    "cancelled" => ExecutionStatus::Cancelled,
                    "timeout" => ExecutionStatus::Timeout,
                    _ => ExecutionStatus::Pending,
                };

                Ok(ExecutionResult {
                    execution_id,
                    workflow_id: row.workflow_id,
                    status,
                    outputs: serde_json::from_value(row.outputs).unwrap_or_default(),
                    error: row.error,
                    started_at: row.started_at,
                    completed_at: row.completed_at,
                    duration_ms: row.duration_ms.map(|d| d as u64),
                    node_executions: vec![],
                })
            }
            None => Err(AutomataError::Execution(
                ExecutionError::ExecutionNotFound {
                    execution_id: execution_id.to_string(),
                },
            )),
        }
    }

    /// List all workflows with pagination
    pub async fn list_workflows(
        &self,
        page: u32,
        limit: u32,
        sort: Option<String>,
        order: Option<String>,
    ) -> Result<(Vec<WorkflowDefinition>, usize)> {
        let offset = page * limit;
        let sort_field = sort.as_deref().unwrap_or("created_at");
        let sort_order = if order.as_deref() == Some("asc") {
            "ASC"
        } else {
            "DESC"
        };

        // Count total workflows
        let total_row = sqlx::query_as::<_, CountRow>(
            r#"
            SELECT COUNT(*) as count
            FROM workflows
            WHERE status = 'active'
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to count workflows: {e}")))?;

        let total = total_row.count;

        // Fetch workflows
        let query = format!(
            r#"
            SELECT id, name, description, definition, version, status, created_at, updated_at, tags, metadata
            FROM workflows
            WHERE status = 'active'
            ORDER BY {sort_field} {sort_order}
            LIMIT $1 OFFSET $2
            "#
        );

        let rows = sqlx::query_as::<_, WorkflowRow>(&query)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AutomataError::Database(format!("Failed to list workflows: {e}")))?;

        let workflows = rows
            .into_iter()
            .map(|row| WorkflowDefinition {
                id: row.id,
                metadata: crate::core::workflow::WorkflowMetadata {
                    name: row.name,
                    version: row.version,
                    description: row.description,
                    tags: row.tags.unwrap_or_default(),
                    author: None,
                    organization: None,
                },
                triggers: vec![],
                nodes: std::collections::HashMap::new(),
                connections: vec![],
                tests: None,
                created_at: row.created_at,
                updated_at: row.updated_at,
            })
            .collect();

        Ok((workflows, total as usize))
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
        let offset = page * limit;
        let sort_field = sort.as_deref().unwrap_or("started_at");
        let sort_order = if order.as_deref() == Some("asc") {
            "ASC"
        } else {
            "DESC"
        };

        // Count total executions
        let total_row = if let Some(wf_id) = workflow_id {
            sqlx::query_as::<_, CountRow>(
                r#"
                SELECT COUNT(*) as count
                FROM executions
                WHERE workflow_id = $1
                "#,
            )
            .bind(wf_id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AutomataError::Database(format!("Failed to count executions: {e}")))?
        } else {
            sqlx::query_as::<_, CountRow>(
                r#"
                SELECT COUNT(*) as count
                FROM executions
                "#,
            )
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AutomataError::Database(format!("Failed to count executions: {e}")))?
        };

        let total = total_row.count;

        // Fetch executions
        let query = if workflow_id.is_some() {
            format!(
                r#"
                SELECT id, workflow_id, status, trigger_data, context, outputs, error, started_at, completed_at, duration_ms
                FROM executions
                WHERE workflow_id = $3
                ORDER BY {sort_field} {sort_order}
                LIMIT $1 OFFSET $2
                "#
            )
        } else {
            format!(
                r#"
                SELECT id, workflow_id, status, trigger_data, context, outputs, error, started_at, completed_at, duration_ms
                FROM executions
                ORDER BY {sort_field} {sort_order}
                LIMIT $1 OFFSET $2
                "#
            )
        };

        let rows = if let Some(wf_id) = workflow_id {
            sqlx::query_as::<_, ExecutionRow>(&query)
                .bind(limit as i64)
                .bind(offset as i64)
                .bind(wf_id)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AutomataError::Database(format!("Failed to list executions: {e}")))?
        } else {
            sqlx::query_as::<_, ExecutionRow>(&query)
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| AutomataError::Database(format!("Failed to list executions: {e}")))?
        };

        let executions = rows
            .into_iter()
            .map(|row| {
                let status = match row.status.as_str() {
                    "pending" => ExecutionStatus::Pending,
                    "running" => ExecutionStatus::Running,
                    "completed" => ExecutionStatus::Completed,
                    "failed" => ExecutionStatus::Failed,
                    "cancelled" => ExecutionStatus::Cancelled,
                    "timeout" => ExecutionStatus::Timeout,
                    _ => ExecutionStatus::Pending,
                };

                ExecutionResult {
                    execution_id: row.id,
                    workflow_id: row.workflow_id,
                    status,
                    outputs: serde_json::from_value(row.outputs).unwrap_or_default(),
                    error: row.error,
                    started_at: row.started_at,
                    completed_at: row.completed_at,
                    duration_ms: row.duration_ms.map(|d| d as u64),
                    node_executions: vec![],
                }
            })
            .collect();

        Ok((executions, total as usize))
    }

    /// Update a workflow definition
    pub async fn update_workflow(
        &self,
        workflow_id: Uuid,
        workflow: &WorkflowDefinition,
    ) -> Result<()> {
        // Convert workflow to YAML
        let definition_yaml = serde_yaml::to_string(&workflow)
            .map_err(|e| AutomataError::Parsing(format!("Failed to serialize workflow: {e}")))?;

        let updated = sqlx::query(
            r#"
            UPDATE workflows
            SET name = $2,
                description = $3,
                definition = $4,
                version = $5,
                tags = $6,
                metadata = $7,
                updated_at = NOW()
            WHERE id = $1 AND status = 'active'
            "#,
        )
        .bind(workflow_id)
        .bind(&workflow.metadata.name)
        .bind(&workflow.metadata.description)
        .bind(definition_yaml)
        .bind(&workflow.metadata.version)
        .bind(&workflow.metadata.tags)
        .bind(serde_json::json!({}))
        .execute(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to update workflow: {e}")))?;

        if updated.rows_affected() == 0 {
            return Err(AutomataError::Execution(ExecutionError::WorkflowNotFound {
                workflow_id: workflow_id.to_string(),
            }));
        }

        Ok(())
    }

    /// Delete a workflow definition
    pub async fn delete_workflow(&self, workflow_id: Uuid) -> Result<()> {
        // Soft delete by setting status to 'deleted'
        let updated = sqlx::query(
            r#"
            UPDATE workflows
            SET status = 'deleted',
                updated_at = NOW()
            WHERE id = $1 AND status = 'active'
            "#,
        )
        .bind(workflow_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AutomataError::Database(format!("Failed to delete workflow: {e}")))?;

        if updated.rows_affected() == 0 {
            return Err(AutomataError::Execution(ExecutionError::WorkflowNotFound {
                workflow_id: workflow_id.to_string(),
            }));
        }

        Ok(())
    }

    /// Get the latest execution for a workflow
    pub async fn get_latest_execution_for_workflow(
        &self,
        workflow_id: Uuid,
    ) -> Option<DateTime<Utc>> {
        sqlx::query_as::<_, MaxDateRow>(
            r#"
            SELECT MAX(started_at) as latest_execution
            FROM executions
            WHERE workflow_id = $1
            "#,
        )
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()
        .and_then(|row| row.latest_execution)
    }
}

#[async_trait::async_trait]
impl StateManagerTrait for PostgresStateManager {
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
