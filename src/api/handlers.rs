//! API request handlers

use crate::api::models::{
    CreateWorkflowRequest, CreateWorkflowResponse, ExecuteWorkflowRequest, ExecuteWorkflowResponse,
    GetExecutionResponse, GetWorkflowResponse, ListNodesResponse, SharedState,
};
use crate::dsl::DslParser;
use axum::{
    extract::{Path, State},
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

    let workflow = state
        .state_manager
        .load_workflow(id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let response = GetWorkflowResponse {
        id: workflow.id,
        metadata: workflow.metadata,
        triggers: workflow.triggers,
        nodes: workflow.nodes,
        connections: workflow.connections,
        tests: workflow.tests,
        created_at: workflow.created_at,
        updated_at: workflow.updated_at,
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
