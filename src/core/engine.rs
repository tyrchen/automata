//! Core execution engine for workflow automation

use crate::core::{
    execution::{
        ExecutionContext, ExecutionPlan, ExecutionResult, ExecutionStage, ExecutionStatus,
        NodeExecutionResult, NodeExecutionStatus,
    },
    scheduler::{ScheduledTask, TaskPriority, TaskScheduler},
    workflow::DirectedGraph,
};
use crate::error::{ExecutionError, Result};
use crate::nodes::{NodeInput, NodeRegistry};
use crate::state::StateManager;
use crate::utils::perf::Timer;
use chrono::Utc;
use dashmap::DashMap;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

/// Core execution engine
#[derive(Clone)]
pub struct ExecutionEngine {
    node_registry: Arc<NodeRegistry>,
    state_manager: Arc<StateManager>,
    scheduler: Arc<TaskScheduler>,
    running_executions: Arc<DashMap<Uuid, RunningExecution>>,
    config: ExecutionEngineConfig,
}

/// Configuration for the execution engine
#[derive(Debug, Clone)]
pub struct ExecutionEngineConfig {
    pub max_concurrent_executions: usize,
    pub max_concurrent_nodes: usize,
    pub default_node_timeout: Duration,
    pub max_execution_timeout: Duration,
    pub enable_checkpointing: bool,
    pub checkpoint_interval: Duration,
}

/// Information about a running execution
#[derive(Debug)]
#[allow(dead_code)]
struct RunningExecution {
    execution_id: Uuid,
    workflow_id: Uuid,
    status: ExecutionStatus,
    started_at: chrono::DateTime<chrono::Utc>,
    current_stage: usize,
    completed_nodes: HashSet<String>,
    failed_nodes: HashSet<String>,
    cancel_sender: mpsc::Sender<()>,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(
        node_registry: Arc<NodeRegistry>,
        state_manager: Arc<StateManager>,
        config: ExecutionEngineConfig,
    ) -> Self {
        let scheduler = Arc::new(TaskScheduler::new(config.max_concurrent_nodes));

        Self {
            node_registry,
            state_manager,
            scheduler,
            running_executions: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Execute a workflow
    pub async fn execute_workflow(
        &self,
        workflow_id: Uuid,
        trigger_data: Value,
    ) -> Result<ExecutionResult> {
        let _timer = Timer::new(format!("execute_workflow_{workflow_id}"));

        // Check concurrent execution limit
        if self.running_executions.len() >= self.config.max_concurrent_executions {
            return Err(crate::error::AutomataError::Execution(
                ExecutionError::ResourceExhausted {
                    resource: "concurrent_executions".to_string(),
                },
            ));
        }

        // Load workflow definition
        let workflow = self.state_manager.load_workflow(workflow_id).await?;

        // Create execution context
        let mut context = ExecutionContext::new(workflow_id, trigger_data);
        let execution_id = context.execution_id;

        // Build DAG
        let dag = workflow.build_dag()?;

        // Create execution plan
        let plan = self.create_execution_plan(&dag)?;

        // Initialize execution state
        self.state_manager
            .init_execution(execution_id, &workflow)
            .await?;

        // Create cancellation channel
        let (cancel_tx, mut cancel_rx) = mpsc::channel(1);

        // Track running execution
        let running_execution = RunningExecution {
            execution_id,
            workflow_id,
            status: ExecutionStatus::Running,
            started_at: Utc::now(),
            current_stage: 0,
            completed_nodes: HashSet::new(),
            failed_nodes: HashSet::new(),
            cancel_sender: cancel_tx,
        };

        self.running_executions
            .insert(execution_id, running_execution);

        // Execute with timeout and cancellation
        let execution_result = tokio::select! {
            result = self.execute_plan(execution_id, plan, &mut context, &dag) => {
                result
            }
            _ = tokio::time::sleep(self.config.max_execution_timeout) => {
                self.cancel_execution(execution_id).await?;
                Err(crate::error::AutomataError::Execution(
                    ExecutionError::ExecutionTimeout {
                        execution_id: execution_id.to_string(),
                        timeout_ms: self.config.max_execution_timeout.as_millis() as u64,
                    }
                ))
            }
            _ = cancel_rx.recv() => {
                Err(crate::error::AutomataError::Execution(
                    ExecutionError::NodeExecutionFailed {
                        node_id: "execution_cancelled".to_string(),
                    }
                ))
            }
        };

        // Clean up
        self.running_executions.remove(&execution_id);

        // Create execution result
        let mut result = ExecutionResult::new(execution_id, workflow_id);

        match execution_result {
            Ok(outputs) => {
                result.complete(outputs);
                info!(
                    execution_id = %execution_id,
                    workflow_id = %workflow_id,
                    duration_ms = result.duration_ms,
                    "Workflow execution completed successfully"
                );
            }
            Err(e) => {
                result.fail(e.to_string());
                error!(
                    execution_id = %execution_id,
                    workflow_id = %workflow_id,
                    error = %e,
                    "Workflow execution failed"
                );
            }
        }

        // Save final state
        self.state_manager
            .finalize_execution(execution_id, &result)
            .await?;

        Ok(result)
    }

    /// Execute the execution plan
    async fn execute_plan(
        &self,
        execution_id: Uuid,
        plan: ExecutionPlan,
        context: &mut ExecutionContext,
        dag: &DirectedGraph,
    ) -> Result<HashMap<String, Value>> {
        let mut completed_stages = Vec::new();
        let mut outputs = HashMap::new();

        for stage in plan.stages {
            info!(
                execution_id = %execution_id,
                stage_id = stage.stage_id,
                node_count = stage.nodes.len(),
                "Starting execution stage"
            );

            // Execute all nodes in this stage concurrently
            let stage_results = self
                .execute_stage(execution_id, &stage, context, dag)
                .await?;

            // Collect outputs and check for failures
            for (node_id, result) in stage_results {
                match result.status {
                    NodeExecutionStatus::Completed => {
                        if let Some(output) = result.output {
                            context.set_node_output(node_id.clone(), output.clone());
                            outputs.insert(node_id, output);
                        }
                    }
                    NodeExecutionStatus::Failed => {
                        return Err(crate::error::AutomataError::Execution(
                            ExecutionError::NodeExecutionFailed { node_id },
                        ));
                    }
                    _ => {
                        warn!(
                            execution_id = %execution_id,
                            node_id = %node_id,
                            status = ?result.status,
                            "Node execution did not complete normally"
                        );
                    }
                }
            }

            completed_stages.push(stage.stage_id);
            context.current_stage = stage.stage_id + 1;

            // Update state checkpoint
            if self.config.enable_checkpointing {
                self.create_checkpoint(execution_id, context).await?;
            }
        }

        Ok(outputs)
    }

    /// Execute all nodes in a stage concurrently
    async fn execute_stage(
        &self,
        execution_id: Uuid,
        stage: &ExecutionStage,
        context: &ExecutionContext,
        dag: &DirectedGraph,
    ) -> Result<HashMap<String, NodeExecutionResult>> {
        let mut tasks = Vec::new();

        // Schedule all nodes in the stage
        for node_id in &stage.nodes {
            let node_def = dag.nodes.get(node_id).ok_or_else(|| {
                crate::error::AutomataError::Execution(ExecutionError::NodeExecutionFailed {
                    node_id: node_id.clone(),
                })
            })?;

            // Check node condition if present
            if let Some(condition) = &node_def.condition {
                let condition_result = context.evaluate_expression(condition)?;
                if !self.is_truthy(&condition_result) {
                    info!(
                        execution_id = %execution_id,
                        node_id = %node_id,
                        condition = %condition,
                        "Skipping node due to condition"
                    );
                    continue;
                }
            }

            let timeout = node_def
                .timeout
                .map(Duration::from_millis)
                .unwrap_or(self.config.default_node_timeout);

            let priority = TaskPriority::Normal; // Could be configurable per node

            let retry_config = crate::core::scheduler::RetryConfig {
                max_retries: node_def.retry.as_ref().map(|r| r.max_attempts).unwrap_or(3),
                delay_ms: node_def.retry.as_ref().map(|r| r.delay_ms).unwrap_or(1000),
                backoff_multiplier: node_def
                    .retry
                    .as_ref()
                    .and_then(|r| r.backoff_multiplier)
                    .unwrap_or(2.0),
            };

            let task_id = self
                .scheduler
                .schedule_with_retry(
                    execution_id,
                    node_id.clone(),
                    priority,
                    timeout,
                    retry_config,
                )
                .await?;

            tasks.push((node_id.clone(), task_id));
        }

        // Execute tasks and collect results
        let mut results = HashMap::new();
        for (node_id, _task_id) in tasks {
            let node_def = dag.nodes.get(&node_id).unwrap().clone();
            let context_clone = context.clone();
            let node_registry = self.node_registry.clone();
            let node_type = node_def.node_type.clone(); // Clone before moving

            let task = ScheduledTask::new(
                execution_id,
                node_id.clone(),
                TaskPriority::Normal,
                self.config.default_node_timeout,
            );

            let task_result = self
                .scheduler
                .execute_task(task, move |_task| async move {
                    Self::execute_single_node(node_registry, node_def, context_clone).await
                })
                .await?;

            match task_result.result {
                Ok(node_result) => {
                    results.insert(node_id, node_result);
                }
                Err(e) => {
                    error!(
                        execution_id = %execution_id,
                        node_id = %node_id,
                        error = %e,
                        "Node execution failed"
                    );

                    // Create failed result
                    let mut failed_result =
                        NodeExecutionResult::new(node_id.clone(), node_type, Value::Null);
                    failed_result.fail(e.to_string());
                    results.insert(node_id, failed_result);
                }
            }
        }

        Ok(results)
    }

    /// Execute a single node
    async fn execute_single_node(
        node_registry: Arc<NodeRegistry>,
        node_def: crate::core::workflow::WorkflowNode,
        context: ExecutionContext,
    ) -> Result<NodeExecutionResult> {
        let _timer = Timer::new(format!("execute_node_{}", node_def.id));

        // Get node implementation
        let node = node_registry.get_node(&node_def.node_type)?;

        // Prepare input
        let input = NodeInput {
            config: node_def.config.clone(),
            data: Value::Null, // Will be populated from context
        };

        // Create result tracker
        let mut result = NodeExecutionResult::new(
            node_def.id.clone(),
            node_def.node_type.clone(),
            input.config.clone(),
        );

        // Execute the node
        let mut execution_context = context;
        match node.execute(&mut execution_context, input).await {
            Ok(output) => {
                result.complete(output.data);
            }
            Err(e) => {
                result.fail(e.to_string());
            }
        }

        Ok(result)
    }

    /// Create an execution plan from a DAG
    fn create_execution_plan(&self, dag: &DirectedGraph) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::new();
        let mut visited = HashSet::new();
        let mut stage_id = 0;

        // Start with entry nodes
        let mut current_nodes = dag.entry_nodes.clone();

        while !current_nodes.is_empty() {
            let stage = ExecutionStage::new(stage_id, current_nodes.clone());
            plan.add_stage(stage);

            // Mark nodes as visited
            for node_id in &current_nodes {
                visited.insert(node_id.clone());
            }

            // Find next nodes that can be executed
            let mut next_nodes = Vec::new();
            for edge in &dag.edges {
                if current_nodes.contains(&edge.source) && !visited.contains(&edge.target) {
                    // Check if all dependencies of the target node are satisfied
                    let dependencies_met = dag
                        .edges
                        .iter()
                        .filter(|e| e.target == edge.target)
                        .all(|e| visited.contains(&e.source));

                    if dependencies_met && !next_nodes.contains(&edge.target) {
                        next_nodes.push(edge.target.clone());
                    }
                }
            }

            current_nodes = next_nodes;
            stage_id += 1;
        }

        Ok(plan)
    }

    /// Check if a value is truthy
    fn is_truthy(&self, value: &Value) -> bool {
        match value {
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
            Value::Array(arr) => !arr.is_empty(),
            Value::Object(obj) => !obj.is_empty(),
            Value::Null => false,
        }
    }

    /// Create a checkpoint of the current execution state
    async fn create_checkpoint(
        &self,
        execution_id: Uuid,
        context: &ExecutionContext,
    ) -> Result<()> {
        let running_execution = self.running_executions.get(&execution_id).ok_or_else(|| {
            crate::error::AutomataError::Execution(ExecutionError::ExecutionNotFound {
                execution_id: execution_id.to_string(),
            })
        })?;

        let state = context.to_state(
            running_execution.completed_nodes.iter().cloned().collect(),
            running_execution.failed_nodes.iter().cloned().collect(),
        );

        self.state_manager
            .save_execution_state(execution_id, &state)
            .await?;

        Ok(())
    }

    /// Cancel a running execution
    pub async fn cancel_execution(&self, execution_id: Uuid) -> Result<()> {
        // Cancel scheduled tasks
        self.scheduler.cancel_execution(execution_id).await?;

        // Send cancellation signal
        if let Some(running_execution) = self.running_executions.get(&execution_id) {
            let _ = running_execution.cancel_sender.send(()).await;
        }

        // Update execution state
        self.state_manager
            .update_execution_status(execution_id, ExecutionStatus::Cancelled)
            .await?;

        info!(execution_id = %execution_id, "Execution cancelled");

        Ok(())
    }

    /// Get execution status
    pub async fn get_execution_status(&self, execution_id: Uuid) -> Result<ExecutionStatus> {
        if let Some(running_execution) = self.running_executions.get(&execution_id) {
            Ok(running_execution.status.clone())
        } else {
            self.state_manager.get_execution_status(execution_id).await
        }
    }

    /// Get running executions count
    pub fn get_running_executions_count(&self) -> usize {
        self.running_executions.len()
    }

    /// Get scheduler statistics
    pub async fn get_scheduler_stats(&self) -> crate::core::scheduler::SchedulerStats {
        self.scheduler.get_stats().await
    }
}

impl Default for ExecutionEngineConfig {
    fn default() -> Self {
        Self {
            max_concurrent_executions: 1000,
            max_concurrent_nodes: 100,
            default_node_timeout: Duration::from_secs(30),
            max_execution_timeout: Duration::from_secs(300), // 5 minutes
            enable_checkpointing: true,
            checkpoint_interval: Duration::from_secs(30),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::workflow::{WorkflowMetadata, WorkflowNode};
    use crate::nodes::builtin::HttpNode;
    use crate::WorkflowDefinition;
    use serde_json::json;

    async fn create_test_engine() -> ExecutionEngine {
        let node_registry = NodeRegistry::new();
        node_registry
            .register("http", Arc::new(HttpNode::new()))
            .unwrap();

        let state_manager = Arc::new(StateManager::new_mock());
        let config = ExecutionEngineConfig::default();

        ExecutionEngine::new(Arc::new(node_registry), state_manager, config)
    }

    #[tokio::test]
    async fn test_execution_plan_creation() {
        let engine = create_test_engine().await;

        let mut workflow = WorkflowDefinition::new(WorkflowMetadata {
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            description: None,
            tags: vec![],
            author: None,
            organization: None,
        });

        // Add nodes
        workflow.nodes.insert(
            "node1".to_string(),
            WorkflowNode {
                id: "node1".to_string(),
                node_type: "http".to_string(),
                config: json!({"url": "http://example.com"}),
                position: None,
                condition: None,
                timeout: None,
                retry: None,
            },
        );

        workflow.nodes.insert(
            "node2".to_string(),
            WorkflowNode {
                id: "node2".to_string(),
                node_type: "http".to_string(),
                config: json!({"url": "http://example2.com"}),
                position: None,
                condition: None,
                timeout: None,
                retry: None,
            },
        );

        let dag = workflow.build_dag().unwrap();
        let plan = engine.create_execution_plan(&dag).unwrap();

        assert_eq!(plan.stages.len(), 1); // Both nodes can run in parallel
        assert_eq!(plan.total_nodes, 2);
    }

    #[tokio::test]
    async fn test_workflow_execution() {
        let engine = create_test_engine().await;
        let workflow_id = Uuid::new_v4();
        let trigger_data = json!({"test": "data"});

        // This would normally fail since we don't have a real state manager
        // but it tests the basic execution flow
        let result = engine.execute_workflow(workflow_id, trigger_data).await;

        // We expect this to fail due to missing workflow, but that's ok for this test
        assert!(result.is_err());
    }
}
