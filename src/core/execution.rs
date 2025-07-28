//! Workflow execution context and state management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Execution context that flows through the workflow
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub trigger_data: Value,
    pub node_outputs: HashMap<String, Value>,
    pub global_variables: HashMap<String, Value>,
    pub environment: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
    pub started_at: DateTime<Utc>,
    pub current_stage: usize,
}

/// Result of workflow execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExecutionResult {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub status: ExecutionStatus,
    pub outputs: HashMap<String, Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub node_executions: Vec<NodeExecutionResult>,
}

/// Status of workflow execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

impl std::fmt::Display for ExecutionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionStatus::Pending => write!(f, "Pending"),
            ExecutionStatus::Running => write!(f, "Running"),
            ExecutionStatus::Completed => write!(f, "Completed"),
            ExecutionStatus::Failed => write!(f, "Failed"),
            ExecutionStatus::Cancelled => write!(f, "Cancelled"),
            ExecutionStatus::Timeout => write!(f, "Timeout"),
        }
    }
}

/// Execution state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub status: ExecutionStatus,
    pub trigger_data: Value,
    pub node_outputs: HashMap<String, Value>,
    pub global_variables: HashMap<String, Value>,
    pub current_stage: usize,
    pub completed_nodes: Vec<String>,
    pub failed_nodes: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Result of individual node execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodeExecutionResult {
    pub node_id: String,
    pub node_type: String,
    pub status: NodeExecutionStatus,
    pub input: Value,
    pub output: Option<Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub retry_count: u32,
}

/// Status of individual node execution
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
    Timeout,
}

/// Execution plan for organizing node execution
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    pub stages: Vec<ExecutionStage>,
    pub total_nodes: usize,
}

/// A stage in the execution plan containing nodes that can run in parallel
#[derive(Debug, Clone)]
pub struct ExecutionStage {
    pub stage_id: usize,
    pub nodes: Vec<String>,
    pub dependencies: Vec<usize>, // Stage IDs this stage depends on
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new(workflow_id: Uuid, trigger_data: Value) -> Self {
        Self {
            execution_id: Uuid::new_v4(),
            workflow_id,
            trigger_data,
            node_outputs: HashMap::new(),
            global_variables: HashMap::new(),
            environment: std::env::vars().collect(),
            secrets: HashMap::new(),
            started_at: Utc::now(),
            current_stage: 0,
        }
    }

    /// Set output for a node
    pub fn set_node_output(&mut self, node_id: String, output: Value) {
        self.node_outputs.insert(node_id, output);
    }

    /// Get output from a node
    pub fn get_node_output(&self, node_id: &str) -> Option<&Value> {
        self.node_outputs.get(node_id)
    }

    /// Set a global variable
    pub fn set_variable(&mut self, key: String, value: Value) {
        self.global_variables.insert(key, value);
    }

    /// Get a global variable
    pub fn get_variable(&self, key: &str) -> Option<&Value> {
        self.global_variables.get(key)
    }

    /// Evaluate an expression in the current context
    pub fn evaluate_expression(&self, expression: &str) -> crate::Result<Value> {
        let mut context = HashMap::new();

        // Add trigger data
        context.insert("trigger".to_string(), self.trigger_data.clone());

        // Add node outputs
        for (node_id, output) in &self.node_outputs {
            context.insert(node_id.clone(), output.clone());
        }

        // Add global variables
        for (key, value) in &self.global_variables {
            context.insert(key.clone(), value.clone());
        }

        // Add environment variables
        let env_vars: serde_json::Map<String, Value> = self
            .environment
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        context.insert("env".to_string(), Value::Object(env_vars));

        // Add secrets (masked for security)
        let secret_vars: serde_json::Map<String, Value> = self
            .secrets
            .keys()
            .map(|k| (k.clone(), Value::String("***".to_string())))
            .collect();
        context.insert("secret".to_string(), Value::Object(secret_vars));

        crate::utils::evaluate_expression(expression, &context)
    }

    /// Get secret value
    pub fn get_secret(&self, key: &str) -> Option<&String> {
        self.secrets.get(key)
    }

    /// Set secret value
    pub fn set_secret(&mut self, key: String, value: String) {
        self.secrets.insert(key, value);
    }

    /// Create execution state for persistence
    pub fn to_state(
        &self,
        completed_nodes: Vec<String>,
        failed_nodes: Vec<String>,
    ) -> ExecutionState {
        ExecutionState {
            execution_id: self.execution_id,
            workflow_id: self.workflow_id,
            status: if failed_nodes.is_empty() {
                ExecutionStatus::Running
            } else {
                ExecutionStatus::Failed
            },
            trigger_data: self.trigger_data.clone(),
            node_outputs: self.node_outputs.clone(),
            global_variables: self.global_variables.clone(),
            current_stage: self.current_stage,
            completed_nodes,
            failed_nodes,
            started_at: self.started_at,
            updated_at: Utc::now(),
        }
    }
}

impl ExecutionResult {
    /// Create a new execution result
    pub fn new(execution_id: Uuid, workflow_id: Uuid) -> Self {
        Self {
            execution_id,
            workflow_id,
            status: ExecutionStatus::Pending,
            outputs: HashMap::new(),
            error: None,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            node_executions: Vec::new(),
        }
    }

    /// Mark execution as completed
    pub fn complete(&mut self, outputs: HashMap<String, Value>) {
        self.status = ExecutionStatus::Completed;
        self.outputs = outputs;
        self.completed_at = Some(Utc::now());
        self.duration_ms = Some(
            self.completed_at
                .unwrap()
                .signed_duration_since(self.started_at)
                .num_milliseconds() as u64,
        );
    }

    /// Mark execution as failed
    pub fn fail(&mut self, error: String) {
        self.status = ExecutionStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(Utc::now());
        self.duration_ms = Some(
            self.completed_at
                .unwrap()
                .signed_duration_since(self.started_at)
                .num_milliseconds() as u64,
        );
    }

    /// Add node execution result
    pub fn add_node_execution(&mut self, result: NodeExecutionResult) {
        self.node_executions.push(result);
    }
}

impl NodeExecutionResult {
    /// Create a new node execution result
    pub fn new(node_id: String, node_type: String, input: Value) -> Self {
        Self {
            node_id,
            node_type,
            status: NodeExecutionStatus::Pending,
            input,
            output: None,
            error: None,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: None,
            retry_count: 0,
        }
    }

    /// Mark node execution as completed
    pub fn complete(&mut self, output: Value) {
        self.status = NodeExecutionStatus::Completed;
        self.output = Some(output);
        self.completed_at = Some(Utc::now());
        self.duration_ms = Some(
            self.completed_at
                .unwrap()
                .signed_duration_since(self.started_at)
                .num_milliseconds() as u64,
        );
    }

    /// Mark node execution as failed
    pub fn fail(&mut self, error: String) {
        self.status = NodeExecutionStatus::Failed;
        self.error = Some(error);
        self.completed_at = Some(Utc::now());
        self.duration_ms = Some(
            self.completed_at
                .unwrap()
                .signed_duration_since(self.started_at)
                .num_milliseconds() as u64,
        );
    }

    /// Increment retry count
    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.status = NodeExecutionStatus::Pending;
        self.completed_at = None;
        self.duration_ms = None;
        self.error = None;
    }
}

impl ExecutionPlan {
    /// Create a new execution plan
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            total_nodes: 0,
        }
    }

    /// Add a stage to the plan
    pub fn add_stage(&mut self, stage: ExecutionStage) {
        self.total_nodes += stage.nodes.len();
        self.stages.push(stage);
    }

    /// Get the next stage to execute
    pub fn get_next_stage(&self, completed_stages: &[usize]) -> Option<&ExecutionStage> {
        for stage in &self.stages {
            // Check if all dependencies are completed
            let dependencies_met = stage
                .dependencies
                .iter()
                .all(|dep_id| completed_stages.contains(dep_id));

            if dependencies_met && !completed_stages.contains(&stage.stage_id) {
                return Some(stage);
            }
        }
        None
    }
}

impl ExecutionStage {
    /// Create a new execution stage
    pub fn new(stage_id: usize, nodes: Vec<String>) -> Self {
        Self {
            stage_id,
            nodes,
            dependencies: Vec::new(),
        }
    }

    /// Add a dependency to this stage
    pub fn add_dependency(&mut self, stage_id: usize) {
        if !self.dependencies.contains(&stage_id) {
            self.dependencies.push(stage_id);
        }
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_execution_context_creation() {
        let workflow_id = Uuid::new_v4();
        let trigger_data = json!({"user_id": 123});

        let context = ExecutionContext::new(workflow_id, trigger_data.clone());

        assert_eq!(context.workflow_id, workflow_id);
        assert_eq!(context.trigger_data, trigger_data);
        assert!(context.node_outputs.is_empty());
    }

    #[test]
    fn test_expression_evaluation() {
        let mut context = ExecutionContext::new(Uuid::new_v4(), json!({"user": "test"}));
        context.set_node_output("validate".to_string(), json!({"success": true}));

        let result = context.evaluate_expression("$trigger.user").unwrap();
        assert_eq!(result, json!("test"));

        let result = context.evaluate_expression("$validate.success").unwrap();
        assert_eq!(result, json!(true));
    }

    #[test]
    fn test_execution_result_lifecycle() {
        let execution_id = Uuid::new_v4();
        let workflow_id = Uuid::new_v4();
        let mut result = ExecutionResult::new(execution_id, workflow_id);

        assert_eq!(result.status, ExecutionStatus::Pending);

        let outputs = [("result".to_string(), json!("success"))]
            .iter()
            .cloned()
            .collect();
        result.complete(outputs);

        assert_eq!(result.status, ExecutionStatus::Completed);
        assert!(result.completed_at.is_some());
        assert!(result.duration_ms.is_some());
    }

    #[test]
    fn test_execution_plan() {
        let mut plan = ExecutionPlan::new();

        let stage1 = ExecutionStage::new(0, vec!["node1".to_string(), "node2".to_string()]);
        let mut stage2 = ExecutionStage::new(1, vec!["node3".to_string()]);
        stage2.add_dependency(0);

        plan.add_stage(stage1);
        plan.add_stage(stage2);

        assert_eq!(plan.total_nodes, 3);

        // Stage 0 should be available first
        let next_stage = plan.get_next_stage(&[]).unwrap();
        assert_eq!(next_stage.stage_id, 0);

        // Stage 1 should be available after stage 0 is completed
        let next_stage = plan.get_next_stage(&[0]).unwrap();
        assert_eq!(next_stage.stage_id, 1);

        // No more stages after both are completed
        let next_stage = plan.get_next_stage(&[0, 1]);
        assert!(next_stage.is_none());
    }
}
