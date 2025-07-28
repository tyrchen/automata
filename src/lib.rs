//! # Automata Workflow Engine
//!
//! A high-performance workflow automation engine built with Rust.
//!
//! ## Features
//!
//! - High-performance async execution (10ms node latency target)
//! - YAML-based DSL with custom extensions
//! - Modular node architecture with built-in and extensible nodes
//! - State management with PostgreSQL and Redis
//! - REST API using Axum
//! - JWT authentication and authorization
//! - Visual workflow support

// Allow large error types for now - this would require significant refactoring to fix
#![allow(clippy::result_large_err)]

pub mod api;
pub mod config;
pub mod core;
pub mod dsl;
pub mod error;
pub mod nodes;
pub mod security;
pub mod state;
pub mod utils;

// Re-export core types for convenience
pub use crate::core::{
    engine::ExecutionEngine,
    execution::{ExecutionContext, ExecutionResult},
    workflow::WorkflowDefinition,
};
pub use crate::error::{AutomataError, Result};
pub use crate::nodes::{Node, NodeInput, NodeOutput, NodeRegistry};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default configuration constants
pub mod defaults {
    pub const DEFAULT_API_PORT: u16 = 8080;
    pub const DEFAULT_WORKER_THREADS: usize = 4;
    pub const DEFAULT_MAX_CONCURRENT_WORKFLOWS: usize = 10000;
    pub const DEFAULT_NODE_TIMEOUT_MS: u64 = 30000;
    pub const DEFAULT_EXECUTION_TIMEOUT_MS: u64 = 300000; // 5 minutes
}
