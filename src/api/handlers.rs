//! API request handlers

use crate::api::models::{
    CancelExecutionResponse, CreateWorkflowRequest, CreateWorkflowResponse, DeleteWorkflowResponse,
    ExecuteWorkflowRequest, ExecuteWorkflowResponse, ExecutionListItem, GetExecutionLogsResponse,
    GetExecutionResponse, GetWorkflowResponse, ListExecutionsQuery, ListExecutionsResponse,
    ListNodesResponse, ListWorkflowsQuery, ListWorkflowsResponse, LogEntry, RerunExecutionResponse,
    SharedState, UpdateWorkflowRequest, UpdateWorkflowResponse, WorkflowListItem,
};
use crate::dsl::DslParser;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use tracing::{error, info};
use uuid::Uuid;

/// Create a new workflow
pub async fn create_workflow(
    State(state): State<SharedState>,
    Json(request): Json<CreateWorkflowRequest>,
) -> Result<Json<CreateWorkflowResponse>, StatusCode> {
    info!("Creating new workflow: {}", request.name);

    // Parse workflow DSL
    let parser = DslParser::new();
    let parsed_workflow = parser.parse(&request.definition).map_err(|e| {
        error!("Failed to parse workflow: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    // Save workflow
    state
        .state_manager
        .save_workflow(&parsed_workflow.definition)
        .await
        .map_err(|e| {
            error!("Failed to save workflow: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = CreateWorkflowResponse {
        id: parsed_workflow.definition.id,
        name: parsed_workflow.definition.metadata.name.clone(),
        status: "created".to_string(),
    };

    info!("Workflow created successfully: {}", response.id);
    Ok(Json(response))
}

/// Get a workflow by ID
pub async fn get_workflow(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetWorkflowResponse>, StatusCode> {
    info!("Getting workflow: {}", id);

    // Try to downcast to PostgresStateManager to get the raw definition
    // If it's not PostgresStateManager, fall back to the regular method
    let (workflow, definition) = if let Some(postgres_state) =
        state
            .state_manager
            .as_any()
            .downcast_ref::<crate::state::postgres::PostgresStateManager>()
    {
        // Use the PostgreSQL-specific method that returns both parsed workflow and raw YAML
        postgres_state
            .load_workflow_with_definition(id)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?
    } else {
        // Fallback for other state managers
        let workflow = state
            .state_manager
            .load_workflow(id)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;

        let definition = serde_yaml::to_string(&workflow)
            .unwrap_or_else(|_| "# Failed to serialize workflow".to_string());

        (workflow, definition)
    };

    let response = GetWorkflowResponse {
        id: workflow.id,
        metadata: workflow.metadata,
        triggers: workflow.triggers,
        nodes: workflow.nodes,
        connections: workflow.connections,
        tests: workflow.tests,
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
        definition,
    };

    Ok(Json(response))
}

/// Execute a workflow
pub async fn execute_workflow(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    Json(request): Json<ExecuteWorkflowRequest>,
) -> Result<Json<ExecuteWorkflowResponse>, StatusCode> {
    info!("Executing workflow: {}", id);

    let result = state
        .execution_engine
        .execute_workflow(id, request.trigger_data)
        .await
        .map_err(|e| {
            error!("Workflow execution failed: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = ExecuteWorkflowResponse {
        execution_id: result.execution_id,
        workflow_id: result.workflow_id,
        status: result.status,
        outputs: if result.outputs.is_empty() {
            None
        } else {
            Some(result.outputs)
        },
        error: result.error,
        started_at: result.started_at,
        completed_at: result.completed_at,
        duration_ms: result.duration_ms.unwrap_or(0),
    };

    info!(
        "Workflow execution completed: {} ({}ms)",
        result.execution_id,
        result.duration_ms.unwrap_or(0)
    );
    Ok(Json(response))
}

/// Get execution status and result
pub async fn get_execution(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetExecutionResponse>, StatusCode> {
    info!("Getting execution: {}", id);

    let result = state
        .state_manager
        .get_execution_result(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let response = GetExecutionResponse {
        execution_id: result.execution_id,
        workflow_id: result.workflow_id,
        status: result.status,
        outputs: if result.outputs.is_empty() {
            None
        } else {
            Some(result.outputs)
        },
        error: result.error,
        started_at: result.started_at,
        completed_at: result.completed_at,
        duration_ms: result.duration_ms.unwrap_or(0),
        node_executions: result.node_executions,
    };

    Ok(Json(response))
}

/// List available nodes
pub async fn list_nodes(
    State(state): State<SharedState>,
) -> Result<Json<ListNodesResponse>, StatusCode> {
    info!("Listing available nodes");

    let nodes = state.node_registry.list_nodes().await;
    let descriptions = state.node_registry.get_all_descriptions_vec().await;

    let total = descriptions.len();
    let response = ListNodesResponse {
        nodes,
        descriptions,
        total,
    };

    Ok(Json(response))
}

/// List workflows with pagination
pub async fn list_workflows(
    State(state): State<SharedState>,
    Query(query): Query<ListWorkflowsQuery>,
) -> Result<Json<ListWorkflowsResponse>, StatusCode> {
    info!("Listing workflows with query: {:?}", query);

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10).min(100); // Cap at 100 items per page

    let (workflows, total) = state
        .state_manager
        .list_workflows(page, limit, query.sort, query.order)
        .await
        .map_err(|e| {
            error!("Failed to list workflows: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut workflow_items = Vec::new();
    for workflow in workflows {
        // Get latest execution time for this workflow
        let last_execution = state
            .state_manager
            .get_latest_execution_for_workflow(workflow.id)
            .await;

        // Calculate node count
        let node_count = workflow.nodes.len();

        // Determine status - for now we'll just set as "active"
        // In a real implementation, you might check if there are any recent failures
        let status = "active".to_string();

        let item = WorkflowListItem {
            id: workflow.id,
            name: workflow.metadata.name.clone(),
            description: workflow.metadata.description.clone(),
            status,
            last_execution,
            node_count,
            created_at: workflow.created_at,
            updated_at: workflow.updated_at,
        };
        workflow_items.push(item);
    }

    let has_next = (page + 1) * limit < total as u32;
    let has_prev = page > 0;

    let response = ListWorkflowsResponse {
        workflows: workflow_items,
        total,
        page,
        limit,
        has_next,
        has_prev,
    };

    info!(
        "Listed {} workflows (total: {})",
        response.workflows.len(),
        total
    );
    Ok(Json(response))
}

/// List executions with pagination
pub async fn list_executions(
    State(state): State<SharedState>,
    Query(query): Query<ListExecutionsQuery>,
) -> Result<Json<ListExecutionsResponse>, StatusCode> {
    info!("Listing executions with query: {:?}", query);

    let page = query.page.unwrap_or(0);
    let limit = query.limit.unwrap_or(10).min(100); // Cap at 100 items per page

    let (executions, total) = state
        .state_manager
        .list_executions(page, limit, query.sort, query.order, query.workflow_id)
        .await
        .map_err(|e| {
            error!("Failed to list executions: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut execution_items = Vec::new();
    for execution in executions {
        // Get workflow name
        let workflow_name = match state
            .state_manager
            .load_workflow(execution.workflow_id)
            .await
        {
            Ok(workflow) => workflow.metadata.name,
            Err(_) => "Unknown Workflow".to_string(),
        };

        let item = ExecutionListItem {
            execution_id: execution.execution_id,
            workflow_id: execution.workflow_id,
            workflow_name,
            status: execution.status,
            started_at: execution.started_at,
            completed_at: execution.completed_at,
            duration_ms: execution.duration_ms.unwrap_or(0),
            error: execution.error,
        };
        execution_items.push(item);
    }

    let has_next = (page + 1) * limit < total as u32;
    let has_prev = page > 0;

    let response = ListExecutionsResponse {
        executions: execution_items,
        total,
        page,
        limit,
        has_next,
        has_prev,
    };

    info!(
        "Listed {} executions (total: {})",
        response.executions.len(),
        total
    );
    Ok(Json(response))
}

/// Update a workflow
pub async fn update_workflow(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateWorkflowRequest>,
) -> Result<Json<UpdateWorkflowResponse>, StatusCode> {
    info!("Updating workflow: {}", id);

    // Load existing workflow
    let mut workflow = state
        .state_manager
        .load_workflow(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Update fields if provided
    if let Some(name) = request.name {
        workflow.metadata.name = name;
    }

    if let Some(description) = request.description {
        workflow.metadata.description = Some(description);
    }

    if let Some(definition) = request.definition {
        // Parse new workflow definition
        let parser = DslParser::new();
        let parsed_workflow = parser.parse(&definition).map_err(|e| {
            error!("Failed to parse workflow definition: {}", e);
            StatusCode::BAD_REQUEST
        })?;

        // Update workflow with new definition while preserving the ID
        workflow = parsed_workflow.definition;
        workflow.id = id; // Preserve the original ID
    }

    // Update timestamp
    workflow.updated_at = chrono::Utc::now();

    // Save updated workflow
    state
        .state_manager
        .update_workflow(id, &workflow)
        .await
        .map_err(|e| {
            error!("Failed to update workflow: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = UpdateWorkflowResponse {
        id: workflow.id,
        name: workflow.metadata.name.clone(),
        status: "updated".to_string(),
        updated_at: workflow.updated_at,
    };

    info!("Workflow updated successfully: {}", id);
    Ok(Json(response))
}

/// Delete a workflow
pub async fn delete_workflow(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DeleteWorkflowResponse>, StatusCode> {
    info!("Deleting workflow: {}", id);

    // Check if workflow exists
    state
        .state_manager
        .load_workflow(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Delete workflow
    state.state_manager.delete_workflow(id).await.map_err(|e| {
        error!("Failed to delete workflow: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let response = DeleteWorkflowResponse {
        id,
        status: "deleted".to_string(),
    };

    info!("Workflow deleted successfully: {}", id);
    Ok(Json(response))
}

/// Cancel an execution
pub async fn cancel_execution(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CancelExecutionResponse>, StatusCode> {
    info!("Cancelling execution: {}", id);

    // Check if execution exists
    let _execution = state
        .state_manager
        .get_execution_result(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Cancel the execution
    state
        .execution_engine
        .cancel_execution(id)
        .await
        .map_err(|e| {
            error!("Failed to cancel execution: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = CancelExecutionResponse {
        execution_id: id,
        status: crate::core::execution::ExecutionStatus::Cancelled,
        cancelled_at: chrono::Utc::now(),
    };

    info!("Execution cancelled successfully: {}", id);
    Ok(Json(response))
}

/// Get execution logs
pub async fn get_execution_logs(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<Json<GetExecutionLogsResponse>, StatusCode> {
    info!("Getting execution logs: {}", id);

    // First verify the execution exists
    let execution = state
        .state_manager
        .get_execution_result(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // For now, generate sample logs based on node executions
    // In a real implementation, these would come from a logging system
    let mut logs = Vec::new();

    // Add start log
    logs.push(LogEntry {
        timestamp: execution.started_at,
        level: "info".to_string(),
        message: format!("Execution started for workflow {}", execution.workflow_id),
        node_id: None,
    });

    // Add logs for each node execution
    for node_exec in &execution.node_executions {
        logs.push(LogEntry {
            timestamp: node_exec.started_at,
            level: "info".to_string(),
            message: format!(
                "Started executing node '{}' of type '{}'",
                node_exec.node_id, node_exec.node_type
            ),
            node_id: Some(node_exec.node_id.clone()),
        });

        match &node_exec.status {
            crate::core::execution::NodeExecutionStatus::Completed => {
                if let Some(completed_at) = node_exec.completed_at {
                    logs.push(LogEntry {
                        timestamp: completed_at,
                        level: "info".to_string(),
                        message: format!("Node '{}' completed successfully", node_exec.node_id),
                        node_id: Some(node_exec.node_id.clone()),
                    });
                }
            }
            crate::core::execution::NodeExecutionStatus::Failed => {
                if let Some(completed_at) = node_exec.completed_at {
                    logs.push(LogEntry {
                        timestamp: completed_at,
                        level: "error".to_string(),
                        message: format!(
                            "Node '{}' failed: {}",
                            node_exec.node_id,
                            node_exec.error.as_deref().unwrap_or("Unknown error")
                        ),
                        node_id: Some(node_exec.node_id.clone()),
                    });
                }
            }
            crate::core::execution::NodeExecutionStatus::Timeout => {
                if let Some(completed_at) = node_exec.completed_at {
                    logs.push(LogEntry {
                        timestamp: completed_at,
                        level: "warn".to_string(),
                        message: format!("Node '{}' timed out", node_exec.node_id),
                        node_id: Some(node_exec.node_id.clone()),
                    });
                }
            }
            _ => {}
        }
    }

    // Add completion log
    if let Some(completed_at) = execution.completed_at {
        let (level, message) = match execution.status {
            crate::core::execution::ExecutionStatus::Completed => {
                ("info", "Execution completed successfully".to_string())
            }
            crate::core::execution::ExecutionStatus::Failed => (
                "error",
                format!(
                    "Execution failed: {}",
                    execution.error.as_deref().unwrap_or("Unknown error")
                ),
            ),
            crate::core::execution::ExecutionStatus::Cancelled => {
                ("warn", "Execution was cancelled".to_string())
            }
            crate::core::execution::ExecutionStatus::Timeout => {
                ("warn", "Execution timed out".to_string())
            }
            _ => ("info", format!("Execution status: {:?}", execution.status)),
        };

        logs.push(LogEntry {
            timestamp: completed_at,
            level: level.to_string(),
            message,
            node_id: None,
        });
    }

    // Sort logs by timestamp
    logs.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

    let response = GetExecutionLogsResponse {
        execution_id: id,
        logs,
    };

    Ok(Json(response))
}

/// Re-run an execution
pub async fn rerun_execution(
    State(state): State<SharedState>,
    Path(id): Path<Uuid>,
) -> Result<Json<RerunExecutionResponse>, StatusCode> {
    info!("Re-running execution: {}", id);

    // Get the original execution to retrieve the workflow and trigger data
    let original_execution = state
        .state_manager
        .get_execution_result(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get the workflow definition
    let workflow = state
        .state_manager
        .load_workflow(original_execution.workflow_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    // Execute the workflow with the same trigger data (using empty data for now)
    let trigger_data = serde_json::json!({});
    let execution_result = state
        .execution_engine
        .execute_workflow(workflow.id, trigger_data)
        .await
        .map_err(|e| {
            error!("Failed to re-run execution: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = RerunExecutionResponse {
        new_execution_id: execution_result.execution_id,
        original_execution_id: id,
        workflow_id: workflow.id,
        status: "started".to_string(),
    };

    info!(
        "Execution re-run successfully: {} -> {}",
        id, execution_result.execution_id
    );
    Ok(Json(response))
}
