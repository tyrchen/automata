//! Node registry for managing available workflow nodes

use crate::error::{NodeError, Result};
use crate::nodes::traits::{Node, NodeCategory, NodeDescription};
use dashmap::DashMap;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

/// Registry for managing workflow nodes
#[derive(Clone)]
pub struct NodeRegistry {
    nodes: Arc<DashMap<String, Arc<dyn Node>>>,
    categories: Arc<DashMap<String, NodeCategory>>,
    descriptions: Arc<DashMap<String, NodeDescription>>,
}

impl NodeRegistry {
    /// Create a new node registry
    pub fn new() -> Self {
        let registry = Self {
            nodes: Arc::new(DashMap::new()),
            categories: Arc::new(DashMap::new()),
            descriptions: Arc::new(DashMap::new()),
        };

        // Register built-in nodes
        registry.register_builtin_nodes();
        registry
    }

    /// Register a node
    pub fn register(&self, node_type: &str, node: Arc<dyn Node>) -> Result<()> {
        // Validate the node
        let description = node.describe();

        // Store the node
        self.nodes.insert(node_type.to_string(), node.clone());
        self.descriptions.insert(node_type.to_string(), description);

        // Categorize the node
        let category = self.determine_category(node_type);
        self.categories.insert(node_type.to_string(), category);

        info!(node_type = %node_type, "Node registered successfully");
        Ok(())
    }

    /// Unregister a node
    pub fn unregister(&self, node_type: &str) -> Result<()> {
        if let Some((_, _node)) = self.nodes.remove(node_type) {
            // Cleanup
            self.descriptions.remove(node_type);
            self.categories.remove(node_type);

            info!(node_type = %node_type, "Node unregistered successfully");
            Ok(())
        } else {
            Err(crate::error::AutomataError::Node(NodeError::NotFound {
                node_type: node_type.to_string(),
            }))
        }
    }

    /// Get a node by type
    pub fn get_node(&self, node_type: &str) -> Result<Arc<dyn Node>> {
        self.nodes
            .get(node_type)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| {
                crate::error::AutomataError::Node(NodeError::NotFound {
                    node_type: node_type.to_string(),
                })
            })
    }

    /// Check if a node type is registered
    pub fn has_node(&self, node_type: &str) -> bool {
        self.nodes.contains_key(node_type)
    }

    /// Get all registered node types
    pub fn get_node_types(&self) -> Vec<String> {
        self.nodes.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get nodes by category
    pub fn get_nodes_by_category(&self, category: NodeCategory) -> Vec<String> {
        self.categories
            .iter()
            .filter_map(|entry| {
                if *entry.value() == category {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get node description
    pub fn get_description(&self, node_type: &str) -> Option<NodeDescription> {
        self.descriptions
            .get(node_type)
            .map(|entry| entry.value().clone())
    }

    /// Get all node descriptions as a map
    pub fn get_all_descriptions(&self) -> HashMap<String, NodeDescription> {
        self.descriptions
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// List all available nodes
    pub async fn list_nodes(&self) -> Vec<String> {
        self.get_node_types()
    }

    /// Get all node descriptions as a vector
    pub async fn get_all_descriptions_vec(&self) -> Vec<NodeDescription> {
        self.descriptions
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get registry statistics
    pub fn get_stats(&self) -> RegistryStats {
        let mut category_counts = HashMap::new();

        for entry in self.categories.iter() {
            let category = entry.value();
            *category_counts.entry(category.clone()).or_insert(0) += 1;
        }

        RegistryStats {
            total_nodes: self.nodes.len(),
            category_counts,
        }
    }

    /// Validate all registered nodes
    pub async fn validate_all(&self) -> Vec<ValidationResult> {
        let mut results = Vec::new();

        for entry in self.nodes.iter() {
            let node_type = entry.key();
            let node = entry.value();

            let result = self.validate_node(node_type, node.clone()).await;
            results.push(result);
        }

        results
    }

    /// Validate a single node
    async fn validate_node(&self, node_type: &str, node: Arc<dyn Node>) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check if node type matches
        if node.node_type() != node_type {
            errors.push(format!(
                "Node type mismatch: registered as '{}', reports as '{}'",
                node_type,
                node.node_type()
            ));
        }

        // Try to initialize the node
        if let Err(e) = node.initialize().await {
            errors.push(format!("Initialization failed: {e}"));
        }

        // Validate description
        let description = node.describe();
        if description.description.is_empty() {
            warnings.push("No description provided".to_string());
        }

        if description.examples.is_empty() {
            warnings.push("No examples provided".to_string());
        }

        ValidationResult {
            node_type: node_type.to_string(),
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Register built-in nodes
    fn register_builtin_nodes(&self) {
        // HTTP node
        let http_node = Arc::new(crate::nodes::builtin::HttpNode::new());
        if let Err(e) = self.register("http", http_node) {
            warn!(error = %e, "Failed to register HTTP node");
        }

        // Validator node
        let validator_node = Arc::new(crate::nodes::builtin::ValidatorNode::new());
        if let Err(e) = self.register("validator", validator_node) {
            warn!(error = %e, "Failed to register Validator node");
        }

        // Transformer node
        let transformer_node = Arc::new(crate::nodes::builtin::TransformerNode::new());
        if let Err(e) = self.register("transformer", transformer_node) {
            warn!(error = %e, "Failed to register Transformer node");
        }

        // Conditional node
        let conditional_node = Arc::new(crate::nodes::builtin::ConditionalNode::new());
        if let Err(e) = self.register("conditional", conditional_node) {
            warn!(error = %e, "Failed to register Conditional node");
        }

        // Database query node
        let db_query_node = Arc::new(crate::nodes::builtin::DatabaseQueryNode::new());
        if let Err(e) = self.register("database_query", db_query_node) {
            warn!(error = %e, "Failed to register Database Query node");
        }

        info!("Built-in nodes registered successfully");
    }

    /// Determine node category based on type
    fn determine_category(&self, node_type: &str) -> NodeCategory {
        match node_type {
            "http" | "webhook" | "api" => NodeCategory::Http,
            "database_query" | "database_insert" | "database_update" | "database_delete" => {
                NodeCategory::Database
            }
            "validator" | "validate" => NodeCategory::Validation,
            "transformer" | "transform" | "map" | "filter" => NodeCategory::Transform,
            "conditional" | "switch" | "foreach" | "parallel" | "wait" => NodeCategory::Control,
            "email" | "sms" | "notification" => NodeCategory::Communication,
            "file" | "s3" | "storage" => NodeCategory::Storage,
            _ => NodeCategory::Custom,
        }
    }

    /// Hot reload a node (unregister and re-register)
    pub async fn reload_node(&self, node_type: &str, node: Arc<dyn Node>) -> Result<()> {
        // Shutdown existing node if it exists
        if let Some(existing_node) = self.nodes.get(node_type) {
            if let Err(e) = existing_node.shutdown().await {
                warn!(
                    node_type = %node_type,
                    error = %e,
                    "Failed to shutdown existing node during reload"
                );
            }
        }

        // Unregister existing node
        let _ = self.unregister(node_type);

        // Register new node
        self.register(node_type, node)?;

        info!(node_type = %node_type, "Node reloaded successfully");
        Ok(())
    }

    /// Get node usage statistics (would be implemented with metrics collection)
    pub fn get_usage_stats(&self) -> HashMap<String, NodeUsageStats> {
        // Placeholder - in a real implementation, this would collect metrics
        HashMap::new()
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_nodes: usize,
    pub category_counts: HashMap<NodeCategory, usize>,
}

/// Node validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub node_type: String,
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Node usage statistics
#[derive(Debug, Clone)]
pub struct NodeUsageStats {
    pub execution_count: u64,
    pub total_duration_ms: u64,
    pub success_rate: f64,
    pub average_duration_ms: f64,
    pub last_executed: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryStats {
    /// Get total nodes by category
    pub fn get_category_count(&self, category: &NodeCategory) -> usize {
        self.category_counts.get(category).copied().unwrap_or(0)
    }

    /// Get all categories with counts
    pub fn get_categories(&self) -> Vec<(NodeCategory, usize)> {
        self.category_counts
            .iter()
            .map(|(cat, count)| (cat.clone(), *count))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nodes::builtin::HttpNode;

    #[test]
    fn test_node_registration() {
        let registry = NodeRegistry::new();
        let http_node = Arc::new(HttpNode::new());

        // Register node
        registry.register("test_http", http_node.clone()).unwrap();
        assert!(registry.has_node("test_http"));

        // Get node
        let retrieved_node = registry.get_node("test_http").unwrap();
        assert_eq!(retrieved_node.node_type(), "http");

        // Unregister node
        registry.unregister("test_http").unwrap();
        assert!(!registry.has_node("test_http"));
    }

    #[test]
    fn test_node_categories() {
        let registry = NodeRegistry::new();

        // Built-in nodes should be categorized
        let http_nodes = registry.get_nodes_by_category(NodeCategory::Http);
        assert!(!http_nodes.is_empty());

        let validator_nodes = registry.get_nodes_by_category(NodeCategory::Validation);
        assert!(!validator_nodes.is_empty());
    }

    #[test]
    fn test_registry_stats() {
        let registry = NodeRegistry::new();
        let stats = registry.get_stats();

        assert!(stats.total_nodes > 0);
        assert!(!stats.category_counts.is_empty());
    }

    #[tokio::test]
    async fn test_node_validation() {
        let registry = NodeRegistry::new();
        let results = registry.validate_all().await;

        // All built-in nodes should be valid
        for result in results {
            assert!(
                result.is_valid,
                "Node {} validation failed: {:?}",
                result.node_type, result.errors
            );
        }
    }
}
