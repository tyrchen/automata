//! Node system for workflow execution

pub mod builtin;
pub mod registry;
pub mod traits;

// Re-export main types
pub use registry::NodeRegistry;
pub use traits::{Node, NodeDescription, NodeInput, NodeOutput, NodeSchema};
