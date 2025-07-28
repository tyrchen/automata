//! Task scheduler for workflow execution

use crate::core::execution::NodeExecutionResult;
use crate::error::{ExecutionError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{sleep, Duration, Instant};
use uuid::Uuid;

/// Task scheduler for managing workflow execution
#[derive(Clone)]
pub struct TaskScheduler {
    task_queue: Arc<RwLock<Vec<ScheduledTask>>>,
    running_tasks: Arc<RwLock<HashMap<Uuid, TaskHandle>>>,
    max_concurrent_tasks: usize,
    current_tasks: Arc<RwLock<usize>>,
}

/// A scheduled task
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduledTask {
    pub id: Uuid,
    pub execution_id: Uuid,
    pub node_id: String,
    pub priority: TaskPriority,
    pub scheduled_at: DateTime<Utc>,
    pub timeout: Duration,
    pub retry_count: u32,
    pub max_retries: u32,
    pub delay_ms: u64,
    pub backoff_multiplier: f64,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum TaskPriority {
    Low = 0,
    #[default]
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            delay_ms: 1000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Handle for a running task
#[derive(Debug)]
pub struct TaskHandle {
    pub task_id: Uuid,
    pub execution_id: Uuid,
    pub node_id: String,
    pub started_at: Instant,
    pub cancel_sender: mpsc::Sender<()>,
}

/// Task execution result
#[derive(Debug)]
pub struct TaskResult {
    pub task_id: Uuid,
    pub result: Result<NodeExecutionResult>,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(max_concurrent_tasks: usize) -> Self {
        Self {
            task_queue: Arc::new(RwLock::new(Vec::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent_tasks,
            current_tasks: Arc::new(RwLock::new(0)),
        }
    }

    /// Schedule a task for execution
    pub async fn schedule_task(&self, task: ScheduledTask) -> Result<()> {
        let mut queue = self.task_queue.write().await;
        queue.push(task);
        // Sort by priority and scheduled time
        queue.sort_by(|a, b| match a.priority.cmp(&b.priority) {
            Ordering::Equal => a.scheduled_at.cmp(&b.scheduled_at),
            other => other,
        });
        Ok(())
    }

    /// Get the next task to execute
    pub async fn get_next_task(&self) -> Option<ScheduledTask> {
        let mut queue = self.task_queue.write().await;

        // Check if we can run more tasks
        let current_count = *self.current_tasks.read().await;
        if current_count >= self.max_concurrent_tasks {
            return None;
        }

        // Get the highest priority task that's ready to run
        let now = Utc::now();
        if let Some(pos) = queue.iter().position(|task| task.scheduled_at <= now) {
            return Some(queue.remove(pos));
        }

        None
    }

    /// Execute a task
    pub async fn execute_task<F, Fut>(&self, task: ScheduledTask, executor: F) -> Result<TaskResult>
    where
        F: FnOnce(ScheduledTask) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<NodeExecutionResult>> + Send + 'static,
    {
        let task_id = task.id;
        let execution_id = task.execution_id;
        let node_id = task.node_id.clone();
        let timeout = task.timeout;

        // Create cancellation channel
        let (cancel_tx, mut cancel_rx) = mpsc::channel(1);

        // Create task handle
        let handle = TaskHandle {
            task_id,
            execution_id,
            node_id: node_id.clone(),
            started_at: Instant::now(),
            cancel_sender: cancel_tx,
        };

        // Add to running tasks
        {
            let mut running = self.running_tasks.write().await;
            running.insert(task_id, handle);

            let mut count = self.current_tasks.write().await;
            *count += 1;
        }

        // Execute the task with timeout and cancellation
        let result = tokio::select! {
            result = executor(task) => {
                result
            }
            _ = sleep(timeout) => {
                Err(crate::error::AutomataError::Execution(
                    ExecutionError::NodeTimeout {
                        node_id: node_id.clone(),
                        timeout_ms: timeout.as_millis() as u64,
                    }
                ))
            }
            _ = cancel_rx.recv() => {
                Err(crate::error::AutomataError::Execution(
                    ExecutionError::NodeExecutionFailed {
                        node_id: node_id.clone(),
                    }
                ))
            }
        };

        // Remove from running tasks
        {
            let mut running = self.running_tasks.write().await;
            running.remove(&task_id);

            let mut count = self.current_tasks.write().await;
            *count -= 1;
        }

        Ok(TaskResult { task_id, result })
    }

    /// Cancel a running task
    pub async fn cancel_task(&self, task_id: Uuid) -> Result<()> {
        let running = self.running_tasks.read().await;
        if let Some(handle) = running.get(&task_id) {
            let _ = handle.cancel_sender.send(()).await;
        }
        Ok(())
    }

    /// Cancel all tasks for an execution
    pub async fn cancel_execution(&self, execution_id: Uuid) -> Result<()> {
        let running = self.running_tasks.read().await;
        let tasks_to_cancel: Vec<_> = running
            .values()
            .filter(|handle| handle.execution_id == execution_id)
            .collect();

        for handle in tasks_to_cancel {
            let _ = handle.cancel_sender.send(()).await;
        }

        Ok(())
    }

    /// Get statistics about the scheduler
    pub async fn get_stats(&self) -> SchedulerStats {
        let queue = self.task_queue.read().await;
        let running = self.running_tasks.read().await;

        SchedulerStats {
            queued_tasks: queue.len(),
            running_tasks: running.len(),
            max_concurrent_tasks: self.max_concurrent_tasks,
        }
    }

    /// Schedule a task with retry logic
    pub async fn schedule_with_retry(
        &self,
        execution_id: Uuid,
        node_id: String,
        priority: TaskPriority,
        timeout: Duration,
        retry_config: RetryConfig,
    ) -> Result<Uuid> {
        let task = ScheduledTask {
            id: Uuid::new_v4(),
            execution_id,
            node_id,
            priority,
            scheduled_at: Utc::now(),
            timeout,
            retry_count: 0,
            max_retries: retry_config.max_retries,
            delay_ms: retry_config.delay_ms,
            backoff_multiplier: retry_config.backoff_multiplier,
        };

        let task_id = task.id;
        self.schedule_task(task).await?;
        Ok(task_id)
    }

    /// Reschedule a failed task for retry
    pub async fn reschedule_for_retry(&self, task: ScheduledTask) -> Result<()> {
        if task.retry_count >= task.max_retries {
            return Err(crate::error::AutomataError::Execution(
                ExecutionError::NodeExecutionFailed {
                    node_id: task.node_id,
                },
            ));
        }

        let delay = Duration::from_millis(
            (task.delay_ms as f64 * task.backoff_multiplier.powi(task.retry_count as i32)) as u64,
        );

        let retry_task = ScheduledTask {
            id: Uuid::new_v4(),
            scheduled_at: Utc::now() + chrono::Duration::from_std(delay).unwrap(),
            retry_count: task.retry_count + 1,
            ..task
        };

        self.schedule_task(retry_task).await
    }

    /// Start the scheduler loop
    pub async fn start(&self) -> mpsc::Receiver<TaskResult> {
        let (_result_tx, result_rx) = mpsc::channel(1000);

        // This would typically be started in a separate task
        // For now, we just return the receiver
        result_rx
    }
}

/// Scheduler statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStats {
    pub queued_tasks: usize,
    pub running_tasks: usize,
    pub max_concurrent_tasks: usize,
}

impl ScheduledTask {
    /// Create a new scheduled task
    pub fn new(
        execution_id: Uuid,
        node_id: String,
        priority: TaskPriority,
        timeout: Duration,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            execution_id,
            node_id,
            priority,
            scheduled_at: Utc::now(),
            timeout,
            retry_count: 0,
            max_retries: 3,
            delay_ms: 1000,
            backoff_multiplier: 2.0,
        }
    }

    /// Create a task with custom retry settings
    pub fn with_retry(
        execution_id: Uuid,
        node_id: String,
        priority: TaskPriority,
        timeout: Duration,
        max_retries: u32,
        delay_ms: u64,
        backoff_multiplier: f64,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            execution_id,
            node_id,
            priority,
            scheduled_at: Utc::now(),
            timeout,
            retry_count: 0,
            max_retries,
            delay_ms,
            backoff_multiplier,
        }
    }

    /// Schedule the task for a specific time
    pub fn scheduled_for(mut self, scheduled_at: DateTime<Utc>) -> Self {
        self.scheduled_at = scheduled_at;
        self
    }
}

// Implement ordering for the priority queue (reverse order for max-heap behavior)
impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // First compare by priority (higher priority first)
        match other.priority.cmp(&self.priority) {
            Ordering::Equal => {
                // Then by scheduled time (earlier first)
                Some(self.scheduled_at.cmp(&other.scheduled_at))
            }
            other_order => Some(other_order),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_priority_ordering() {
        let low_task = ScheduledTask::new(
            Uuid::new_v4(),
            "low".to_string(),
            TaskPriority::Low,
            Duration::from_secs(30),
        );

        let high_task = ScheduledTask::new(
            Uuid::new_v4(),
            "high".to_string(),
            TaskPriority::High,
            Duration::from_secs(30),
        );

        let critical_task = ScheduledTask::new(
            Uuid::new_v4(),
            "critical".to_string(),
            TaskPriority::Critical,
            Duration::from_secs(30),
        );

        // Test priority comparison
        assert!(critical_task.priority > high_task.priority);
        assert!(high_task.priority > low_task.priority);
        assert_eq!(low_task.priority, TaskPriority::Low);
        assert_eq!(high_task.priority, TaskPriority::High);
        assert_eq!(critical_task.priority, TaskPriority::Critical);
    }

    #[tokio::test]
    async fn test_scheduler_basic_operations() {
        let scheduler = TaskScheduler::new(5);

        let task = ScheduledTask::new(
            Uuid::new_v4(),
            "test_node".to_string(),
            TaskPriority::Normal,
            Duration::from_secs(30),
        );

        // Schedule task
        scheduler.schedule_task(task.clone()).await.unwrap();

        // Get next task
        let next_task = scheduler.get_next_task().await;
        assert!(next_task.is_some());
        assert_eq!(next_task.unwrap().id, task.id);

        // Check stats
        let stats = scheduler.get_stats().await;
        assert_eq!(stats.max_concurrent_tasks, 5);
    }

    #[tokio::test]
    async fn test_task_cancellation() {
        let scheduler = TaskScheduler::new(5);
        let execution_id = Uuid::new_v4();

        let _task_id = scheduler
            .schedule_with_retry(
                execution_id,
                "test_node".to_string(),
                TaskPriority::Normal,
                Duration::from_secs(30),
                RetryConfig::default(),
            )
            .await
            .unwrap();

        // Cancel the execution
        scheduler.cancel_execution(execution_id).await.unwrap();
    }
}
