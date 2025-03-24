//! Data models for the Operational State Instance
//!
//! This file contains all the data structures used by the Operational State Instance.

use std::collections::{HashMap, VecDeque, HashSet};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc, Duration};

use crate::core::component::{ComponentTask, ComponentMessage, ComponentConfig};

/// Operational modes for the instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationalMode {
    /// Focused on solving specific problems
    ProblemSolving,
    /// Focused on exploring knowledge and possibilities
    Exploration,
    /// Focused on memory consolidation and organization
    Consolidation,
    /// Focused on future planning and goal setting
    Planning,
    /// Custom mode with specific settings
    Custom(String),
}

/// Configuration for an operational mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationalModeConfig {
    /// Mode type
    pub mode: OperationalMode,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Default resource allocation
    pub default_allocation: HashMap<String, f64>,
    /// Default task priorities by category
    pub default_priorities: HashMap<String, u8>,
    /// Mode-specific settings
    pub settings: serde_json::Value,
}

/// Task with priority information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedTask {
    /// Underlying task
    pub task: ComponentTask,
    /// Task category (e.g., "memory", "reasoning", "perception")
    pub category: String,
    /// Priority level (higher is more important)
    pub priority: u8,
    /// Required resources
    pub required_resources: HashMap<String, f64>,
    /// Estimated duration in seconds
    pub estimated_duration: f64,
    /// Whether this is a system task
    pub is_system_task: bool,
    /// Whether this task can be interrupted
    pub interruptible: bool,
    /// Time when task was queued
    pub queued_at: DateTime<Utc>,
}

/// Information about a running task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunningTaskInfo {
    /// Task identifier
    pub task_id: String,
    /// Task description
    pub description: String,
    /// Task category
    pub category: String,
    /// Start time
    pub started_at: DateTime<Utc>,
    /// Expected completion time
    pub expected_completion: DateTime<Utc>,
    /// Allocated resources
    pub allocated_resources: HashMap<String, f64>,
    /// Task progress (0-1)
    pub progress: f64,
    /// Whether task is interruptible
    pub interruptible: bool,
    /// Last state snapshot (for recovery)
    pub state_snapshot: Option<serde_json::Value>,
    /// Time of last snapshot
    pub last_snapshot_time: DateTime<Utc>,
}

/// Task scheduled for future execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledTask {
    /// Prioritized task to execute
    pub task: PrioritizedTask,
    /// Scheduled execution time
    pub scheduled_time: DateTime<Utc>,
    /// Recurrence pattern (if any)
    pub recurrence: Option<RecurrencePattern>,
    /// Number of times this task has executed (for recurring tasks)
    pub execution_count: u32,
}

/// Recurrence pattern for scheduled tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecurrencePattern {
    /// Run every N seconds
    EverySeconds(u64),
    /// Run every N minutes
    EveryMinutes(u64),
    /// Run every N hours
    EveryHours(u64),
    /// Run every N days
    EveryDays(u64),
    /// Run on specific days of week
    WeeklyOnDays(Vec<u8>),
    /// Run on specific days of month
    MonthlyOnDays(Vec<u8>),
    /// Custom recurrence pattern
    Custom(String),
}

/// Resource allocation settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    /// Total available resources
    pub total_resources: HashMap<String, f64>,
    /// Currently allocated resources
    pub allocated_resources: HashMap<String, f64>,
    /// Allocation policies
    pub allocation_policies: HashMap<String, AllocationPolicy>,
    /// Resource reservation for system tasks
    pub system_reservation: HashMap<String, f64>,
}

/// Allocation policy for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllocationPolicy {
    /// Equal allocation to all requesting tasks
    EqualShare,
    /// Allocation based on task priority
    PriorityBased,
    /// First-come-first-served
    FCFS,
    /// Custom allocation strategy
    Custom(String),
}

/// Entry in the task history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHistoryEntry {
    /// Task identifier
    pub task_id: String,
    /// Task description
    pub description: String,
    /// Task category
    pub category: String,
    /// Priority level
    pub priority: u8,
    /// Time when task was queued
    pub queued_at: DateTime<Utc>,
    /// Time when task started
    pub started_at: DateTime<Utc>,
    /// Time when task completed
    pub completed_at: DateTime<Utc>,
    /// Task status
    pub status: TaskStatus,
    /// Resources used
    pub resources_used: HashMap<String, f64>,
    /// Result summary (if available)
    pub result_summary: Option<String>,
}

/// Status of a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// Successfully completed
    Completed,
    /// Failed to complete
    Failed,
    /// Interrupted
    Interrupted,
    /// Timed out
    TimedOut,
    /// Cancelled
    Cancelled,
}

/// Goal for autonomous operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    /// Goal identifier
    pub id: String,
    /// Goal description
    pub description: String,
    /// Goal priority (higher is more important)
    pub priority: u8,
    /// Time when goal was created
    pub created_at: DateTime<Utc>,
    /// Expected completion time (if any)
    pub deadline: Option<DateTime<Utc>>,
    /// Specific targets to achieve
    pub targets: Vec<GoalTarget>,
    /// Current status
    pub status: GoalStatus,
    /// Related tasks
    pub related_tasks: Vec<String>,
    /// Parent goal (if any)
    pub parent_goal: Option<String>,
    /// Child goals (if any)
    pub child_goals: Vec<String>,
}

/// Target for a goal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalTarget {
    /// Target identifier
    pub id: String,
    /// Target description
    pub description: String,
    /// Measurement criteria
    pub criteria: serde_json::Value,
    /// Current progress (0-1)
    pub progress: f64,
    /// Current status
    pub status: GoalStatus,
}

/// Status of a goal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    /// Not yet started
    Pending,
    /// In progress
    InProgress,
    /// Completed successfully
    Completed,
    /// Will not be pursued further
    Abandoned,
    /// Failed to achieve
    Failed,
}

/// Settings for autonomous operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousSettings {
    /// Whether autonomous operation is enabled
    pub enabled: bool,
    /// Operational mode switching strategy
    pub mode_switching: ModeSwitchingStrategy,
    /// Minimum time in a mode before switching
    pub min_time_in_mode_secs: u64,
    /// Goal generation settings
    pub goal_generation: GoalGenerationSettings,
    /// Resource thresholds for mode switching
    pub resource_thresholds: HashMap<String, f64>,
    /// Task generation settings
    pub task_generation: TaskGenerationSettings,
}

/// Strategy for switching operational modes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ModeSwitchingStrategy {
    /// Fixed timing
    TimeBased {
        /// Rotation schedule in seconds
        schedule: Vec<(OperationalMode, u64)>,
    },
    /// Based on system state
    StateBased {
        /// Conditions for each mode
        conditions: HashMap<String, serde_json::Value>,
    },
    /// Based on resource usage
    ResourceBased,
    /// Based on active goals
    GoalBased,
    /// Manually controlled
    Manual,
}

/// Settings for goal generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalGenerationSettings {
    /// Whether automatic goal generation is enabled
    pub enabled: bool,
    /// Maximum number of active goals
    pub max_active_goals: usize,
    /// Minimum importance for generating a goal
    pub min_importance: f64,
    /// Generation frequency in seconds
    pub frequency_secs: u64,
}

/// Settings for task generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskGenerationSettings {
    /// Whether automatic task generation is enabled
    pub enabled: bool,
    /// Task templates by category
    pub templates: HashMap<String, Vec<TaskTemplate>>,
    /// Generation frequency in seconds
    pub frequency_secs: u64,
}

/// Template for task generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    /// Template identifier
    pub id: String,
    /// Template description
    pub description: String,
    /// Task parameters
    pub parameters: serde_json::Value,
    /// Conditions for generating this task
    pub conditions: Option<serde_json::Value>,
    /// Priority
    pub default_priority: u8,
    /// Resource requirements
    pub resource_requirements: HashMap<String, f64>,
}

/// Statistics for operational state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OperationalStats {
    /// Total tasks processed
    pub total_tasks_processed: u64,
    /// Tasks processed by category
    pub tasks_by_category: HashMap<String, u64>,
    /// Tasks processed by status
    pub tasks_by_status: HashMap<String, u64>,
    /// Total operational time in seconds
    pub total_operational_time_secs: u64,
    /// Time spent in each mode
    pub time_by_mode: HashMap<String, u64>,
    /// Average task queue size
    pub avg_queue_size: f64,
    /// Average task waiting time
    pub avg_waiting_time_secs: f64,
    /// Average resource utilization
    pub avg_resource_utilization: HashMap<String, f64>,
    /// Mode switches
    pub mode_switches: u64,
    /// Goal statistics
    pub goal_stats: GoalStats,
}

/// Statistics about goals
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GoalStats {
    /// Total goals created
    pub total_goals: u64,
    /// Goals by status
    pub goals_by_status: HashMap<String, u64>,
    /// Average goal completion time
    pub avg_completion_time_secs: f64,
    /// Goal success rate
    pub success_rate: f64,
}

/// Data for recovering from interruptions
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RecoveryData {
    /// Last operational mode
    pub last_mode: Option<OperationalMode>,
    /// Tasks in progress at time of interruption
    pub interrupted_tasks: Vec<RunningTaskInfo>,
    /// Pending high-priority tasks
    pub pending_tasks: Vec<PrioritizedTask>,
    /// Time of interruption
    pub interruption_time: Option<DateTime<Utc>>,
    /// Recovery state
    pub recovery_state: RecoveryState,
    /// Interrupted goals
    pub interrupted_goals: Vec<Goal>,
}

/// State of recovery
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecoveryState {
    /// No recovery needed
    None,
    /// Recovery needed
    Needed,
    /// Recovery in progress
    InProgress,
    /// Recovery completed
    Completed,
    /// Recovery failed
    Failed,
}

impl Default for RecoveryState {
    fn default() -> Self {
        RecoveryState::None
    }
}