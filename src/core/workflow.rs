//! Workflow definition and metadata structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

/// Complete workflow definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowDefinition {
    pub id: Uuid,
    pub metadata: WorkflowMetadata,
    pub triggers: Vec<WorkflowTrigger>,
    pub nodes: HashMap<String, WorkflowNode>,
    pub connections: Vec<WorkflowConnection>,
    pub tests: Option<Vec<WorkflowTest>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Workflow metadata
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowMetadata {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub organization: Option<String>,
}

/// Workflow trigger definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type")]
pub enum WorkflowTrigger {
    #[serde(rename = "webhook")]
    Webhook {
        path: String,
        method: String,
        auth: Option<String>,
        headers: Option<HashMap<String, String>>,
    },
    #[serde(rename = "schedule")]
    Schedule {
        cron: String,
        timezone: Option<String>,
    },
    #[serde(rename = "event")]
    Event {
        source: String,
        event_type: String,
        filters: Option<HashMap<String, Value>>,
    },
    #[serde(rename = "manual")]
    Manual {
        parameters: Option<HashMap<String, Value>>,
    },
}

/// Individual workflow node
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowNode {
    pub id: String,
    pub node_type: String,
    pub config: Value,
    pub position: Option<NodePosition>,
    pub condition: Option<String>,
    pub timeout: Option<u64>,
    pub retry: Option<RetryConfig>,
}

/// Node position for visual representation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

/// Retry configuration for nodes
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub delay_ms: u64,
    pub backoff_multiplier: Option<f64>,
    pub max_delay_ms: Option<u64>,
}

/// Connection between nodes
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowConnection {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
    pub label: Option<String>,
}

/// Test case for workflow
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WorkflowTest {
    pub name: String,
    pub input: Value,
    pub mocks: Option<HashMap<String, Value>>,
    pub expected: HashMap<String, Value>,
}

/// Directed Acyclic Graph representation
#[derive(Debug, Clone)]
pub struct DirectedGraph {
    pub nodes: HashMap<String, WorkflowNode>,
    pub edges: Vec<GraphEdge>,
    pub entry_nodes: Vec<String>,
}

/// Graph edge
#[derive(Debug, Clone)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub condition: Option<String>,
    pub label: Option<String>,
}

impl WorkflowDefinition {
    /// Create a new workflow definition
    pub fn new(metadata: WorkflowMetadata) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            metadata,
            triggers: Vec::new(),
            nodes: HashMap::new(),
            connections: Vec::new(),
            tests: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Build a directed graph from the workflow definition
    pub fn build_dag(&self) -> crate::Result<DirectedGraph> {
        let mut graph = DirectedGraph {
            nodes: self.nodes.clone(),
            edges: Vec::new(),
            entry_nodes: Vec::new(),
        };

        // Convert connections to graph edges
        for conn in &self.connections {
            graph.edges.push(GraphEdge {
                source: conn.from.clone(),
                target: conn.to.clone(),
                condition: conn.condition.clone(),
                label: conn.label.clone(),
            });
        }

        // Find entry nodes (nodes with no incoming edges)
        let mut has_incoming: HashMap<String, bool> = HashMap::new();
        for node_id in self.nodes.keys() {
            has_incoming.insert(node_id.clone(), false);
        }

        for edge in &graph.edges {
            has_incoming.insert(edge.target.clone(), true);
        }

        graph.entry_nodes = has_incoming
            .into_iter()
            .filter_map(|(node_id, has_inc)| if !has_inc { Some(node_id) } else { None })
            .collect();

        // Check for cycles
        self.validate_dag(&graph)?;

        Ok(graph)
    }

    /// Validate that the graph is acyclic
    fn validate_dag(&self, graph: &DirectedGraph) -> crate::Result<()> {
        use std::collections::HashSet;

        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node_id in graph.nodes.keys() {
            if !visited.contains(node_id)
                && Self::has_cycle_util(node_id, graph, &mut visited, &mut rec_stack)?
            {
                return Err(crate::error::AutomataError::DslParse(
                    crate::error::DslError::CircularDependency {
                        nodes: rec_stack.into_iter().collect(),
                    },
                ));
            }
        }

        Ok(())
    }

    /// Utility function for cycle detection using DFS
    fn has_cycle_util(
        node_id: &str,
        graph: &DirectedGraph,
        visited: &mut std::collections::HashSet<String>,
        rec_stack: &mut std::collections::HashSet<String>,
    ) -> crate::Result<bool> {
        visited.insert(node_id.to_string());
        rec_stack.insert(node_id.to_string());

        // Find all outgoing edges from this node
        for edge in &graph.edges {
            if edge.source == node_id {
                if !visited.contains(&edge.target) {
                    if Self::has_cycle_util(&edge.target, graph, visited, rec_stack)? {
                        return Ok(true);
                    }
                } else if rec_stack.contains(&edge.target) {
                    return Ok(true);
                }
            }
        }

        rec_stack.remove(node_id);
        Ok(false)
    }

    /// Get nodes that can be executed given the current state
    pub fn get_executable_nodes(
        &self,
        graph: &DirectedGraph,
        completed_nodes: &std::collections::HashSet<String>,
    ) -> Vec<String> {
        let mut executable = Vec::new();

        for node_id in graph.nodes.keys() {
            if completed_nodes.contains(node_id) {
                continue;
            }

            // Check if all dependencies are satisfied
            let mut dependencies_met = true;
            for edge in &graph.edges {
                if edge.target == *node_id && !completed_nodes.contains(&edge.source) {
                    dependencies_met = false;
                    break;
                }
            }

            if dependencies_met {
                executable.push(node_id.clone());
            }
        }

        executable
    }

    /// Update metadata timestamp
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            delay_ms: 1000,
            backoff_multiplier: Some(2.0),
            max_delay_ms: Some(30000),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_workflow_creation() {
        let metadata = WorkflowMetadata {
            name: "Test Workflow".to_string(),
            version: "1.0.0".to_string(),
            description: Some("A test workflow".to_string()),
            tags: vec!["test".to_string()],
            author: Some("test@example.com".to_string()),
            organization: None,
        };

        let workflow = WorkflowDefinition::new(metadata);
        assert_eq!(workflow.metadata.name, "Test Workflow");
        assert_eq!(workflow.metadata.version, "1.0.0");
    }

    #[test]
    fn test_dag_creation() {
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
                node_type: "transformer".to_string(),
                config: json!({"mapping": {}}),
                position: None,
                condition: None,
                timeout: None,
                retry: None,
            },
        );

        // Add connection
        workflow.connections.push(WorkflowConnection {
            from: "node1".to_string(),
            to: "node2".to_string(),
            condition: None,
            label: None,
        });

        let dag = workflow.build_dag().unwrap();
        assert_eq!(dag.nodes.len(), 2);
        assert_eq!(dag.edges.len(), 1);
        assert_eq!(dag.entry_nodes, vec!["node1".to_string()]);
    }

    #[test]
    fn test_cycle_detection() {
        let mut workflow = WorkflowDefinition::new(WorkflowMetadata {
            name: "Cyclic".to_string(),
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
                node_type: "test".to_string(),
                config: json!({}),
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
                node_type: "test".to_string(),
                config: json!({}),
                position: None,
                condition: None,
                timeout: None,
                retry: None,
            },
        );

        // Create cycle
        workflow.connections.push(WorkflowConnection {
            from: "node1".to_string(),
            to: "node2".to_string(),
            condition: None,
            label: None,
        });

        workflow.connections.push(WorkflowConnection {
            from: "node2".to_string(),
            to: "node1".to_string(),
            condition: None,
            label: None,
        });

        let result = workflow.build_dag();
        assert!(result.is_err());
    }
}
