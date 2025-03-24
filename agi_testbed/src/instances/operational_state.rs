use std::any::Any;
use std::collections::{HashMap, VecDeque, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use async_trait::async_trait;
use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use serde_json::json;
use chrono::{DateTime, Utc, Duration};

use crate::core::component::{
    Component, ComponentStatus, ComponentMessage, ComponentTask, 
    ComponentError, ComponentMetrics, ComponentConfig
};
use crate::instances::common::{
    BaseComponent, EventType, measure_execution_time, extract_task_param
};

/// Persistent Operational State Instance
///
/// This component demonstrates autonomous operation in multiple modes:
/// problem-solving, exploration, consolidation, and planning.
/// It balances resources among concurrent tasks and maintains
/// continuous operation with various priorities.
#[derive(Debug)]
pub struct OperationalStateInstance {
    /// Base component functionality
    base: BaseComponent,
    /// Current operational mode
    mode: Arc<RwLock<OperationalMode>>,
    /// Available operational modes
    available_modes: Vec<OperationalModeConfig>,
    /// Task queue for scheduling
    task_queue: Arc<Mutex<VecDeque<PrioritizedTask>>>,
    /// Currently running tasks
    running_tasks: Arc<Mutex<HashMap<String, RunningTaskInfo>>>,
    /// Scheduled tasks for future execution
    scheduled_tasks: Arc<Mutex<Vec<ScheduledTask>>>,
    /// Resource allocation settings
    resource_allocation: ResourceAllocation,
    /// Task history
    task_history: Arc<Mutex<VecDeque<TaskHistoryEntry>>>,
    /// Current goals
    goals: Arc<RwLock<Vec<Goal>>>,
    /// Autonomous operation settings
    autonomous_settings: AutonomousSettings,
    /// Stats tracking
    stats: Arc<RwLock<OperationalStats>>,
    /// Interruption recovery data
    recovery_data: Arc<RwLock<RecoveryData>>,
}

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

impl OperationalStateInstance {
    /// Create a new operational state instance
    pub fn new() -> Self {
        // Define default available modes
        let available_modes = vec![
            OperationalModeConfig {
                mode: OperationalMode::ProblemSolving,
                name: "Problem Solving".to_string(),
                description: "Focused on solving specific problems".to_string(),
                default_allocation: HashMap::from([
                    ("cpu".to_string(), 0.7),
                    ("memory".to_string(), 0.6),
                    ("attention".to_string(), 0.8),
                    ("creativity".to_string(), 0.4),
                ]),
                default_priorities: HashMap::from([
                    ("reasoning".to_string(), 10),
                    ("memory".to_string(), 8),
                    ("perception".to_string(), 7),
                    ("exploration".to_string(), 3),
                ]),
                settings: json!({
                    "max_problem_complexity": 8,
                    "detailed_solution_generation": true,
                    "solution_verification": true
                }),
            },
            OperationalModeConfig {
                mode: OperationalMode::Exploration,
                name: "Exploration".to_string(),
                description: "Focused on exploring knowledge and possibilities".to_string(),
                default_allocation: HashMap::from([
                    ("cpu".to_string(), 0.5),
                    ("memory".to_string(), 0.7),
                    ("attention".to_string(), 0.5),
                    ("creativity".to_string(), 0.9),
                ]),
                default_priorities: HashMap::from([
                    ("reasoning".to_string(), 6),
                    ("memory".to_string(), 7),
                    ("perception".to_string(), 9),
                    ("exploration".to_string(), 10),
                ]),
                settings: json!({
                    "exploration_breadth": 0.8,
                    "novelty_threshold": 0.6,
                    "connection_forming": true
                }),
            },
            OperationalModeConfig {
                mode: OperationalMode::Consolidation,
                name: "Consolidation".to_string(),
                description: "Focused on memory consolidation and organization".to_string(),
                default_allocation: HashMap::from([
                    ("cpu".to_string(), 0.6),
                    ("memory".to_string(), 0.9),
                    ("attention".to_string(), 0.4),
                    ("creativity".to_string(), 0.3),
                ]),
                default_priorities: HashMap::from([
                    ("reasoning".to_string(), 7),
                    ("memory".to_string(), 10),
                    ("perception".to_string(), 3),
                    ("exploration".to_string(), 5),
                ]),
                settings: json!({
                    "consolidation_depth": 0.9,
                    "redundancy_elimination": true,
                    "structural_reorganization": true
                }),
            },
            OperationalModeConfig {
                mode: OperationalMode::Planning,
                name: "Planning".to_string(),
                description: "Focused on future planning and goal setting".to_string(),
                default_allocation: HashMap::from([
                    ("cpu".to_string(), 0.8),
                    ("memory".to_string(), 0.6),
                    ("attention".to_string(), 0.7),
                    ("creativity".to_string(), 0.7),
                ]),
                default_priorities: HashMap::from([
                    ("reasoning".to_string(), 9),
                    ("memory".to_string(), 7),
                    ("perception".to_string(), 4),
                    ("exploration".to_string(), 6),
                ]),
                settings: json!({
                    "planning_horizon_secs": 3600 * 24 * 7, // One week
                    "goal_decomposition": true,
                    "contingency_planning": true
                }),
            },
        ];
        
        // Define default resource allocation
        let resource_allocation = ResourceAllocation {
            total_resources: HashMap::from([
                ("cpu".to_string(), 1.0),
                ("memory".to_string(), 1.0),
                ("attention".to_string(), 1.0),
                ("creativity".to_string(), 1.0),
            ]),
            allocated_resources: HashMap::from([
                ("cpu".to_string(), 0.0),
                ("memory".to_string(), 0.0),
                ("attention".to_string(), 0.0),
                ("creativity".to_string(), 0.0),
            ]),
            allocation_policies: HashMap::from([
                ("cpu".to_string(), AllocationPolicy::PriorityBased),
                ("memory".to_string(), AllocationPolicy::PriorityBased),
                ("attention".to_string(), AllocationPolicy::PriorityBased),
                ("creativity".to_string(), AllocationPolicy::PriorityBased),
            ]),
            system_reservation: HashMap::from([
                ("cpu".to_string(), 0.1),
                ("memory".to_string(), 0.1),
                ("attention".to_string(), 0.1),
                ("creativity".to_string(), 0.1),
            ]),
        };
        
        // Define default autonomous settings
        let autonomous_settings = AutonomousSettings {
            enabled: true,
            mode_switching: ModeSwitchingStrategy::TimeBased {
                schedule: vec![
                    (OperationalMode::ProblemSolving, 1800),    // 30 min
                    (OperationalMode::Exploration, 1200),       // 20 min
                    (OperationalMode::Consolidation, 900),      // 15 min
                    (OperationalMode::Planning, 1200),          // 20 min
                ],
            },
            min_time_in_mode_secs: 300, // 5 minutes
            goal_generation: GoalGenerationSettings {
                enabled: true,
                max_active_goals: 5,
                min_importance: 0.6,
                frequency_secs: 3600, // 1 hour
            },
            resource_thresholds: HashMap::from([
                ("cpu".to_string(), 0.9),
                ("memory".to_string(), 0.9),
                ("attention".to_string(), 0.9),
                ("creativity".to_string(), 0.9),
            ]),
            task_generation: TaskGenerationSettings {
                enabled: true,
                templates: HashMap::new(),  // Would be populated with actual templates
                frequency_secs: 300, // 5 minutes
            },
        };
        
        Self {
            base: BaseComponent::new("operational_state", "OperationalStateInstance"),
            mode: Arc::new(RwLock::new(OperationalMode::ProblemSolving)),
            available_modes,
            task_queue: Arc::new(Mutex::new(VecDeque::new())),
            running_tasks: Arc::new(Mutex::new(HashMap::new())),
            scheduled_tasks: Arc::new(Mutex::new(Vec::new())),
            resource_allocation,
            task_history: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            goals: Arc::new(RwLock::new(Vec::new())),
            autonomous_settings,
            stats: Arc::new(RwLock::new(OperationalStats::default())),
            recovery_data: Arc::new(RwLock::new(RecoveryData::default())),
        }
    }
    
    /// Switch to a different operational mode
    pub async fn switch_mode(&self, mode: OperationalMode) -> Result<(), ComponentError> {
        let current_mode = { self.mode.read().await.clone() };
        
        if current_mode == mode {
            debug!("Already in {} mode, no switch needed", self.get_mode_name(&current_mode));
            return Ok(());
        }
        
        // Validate mode
        if !self.available_modes.iter().any(|m| m.mode == mode) {
            return Err(ComponentError::ValidationError(format!(
                "Unsupported operational mode: {:?}", mode
            )));
        }
        
        debug!("Switching from {} mode to {} mode", 
              self.get_mode_name(&current_mode), 
              self.get_mode_name(&mode));
        
        // Capture state of current mode for possible later restoration
        self.capture_mode_state(&current_mode).await?;
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.mode_switches += 1;
            
            // Update time in previous mode
            let mode_name = self.get_mode_name(&current_mode);
            let current_time = stats.time_by_mode.entry(mode_name).or_insert(0);
            *current_time += 60; // Approximation, in a real system we'd track actual time
        }
        
        // Set the new mode
        *self.mode.write().await = mode.clone();
        
        // Apply mode-specific resource allocations
        if let Some(mode_config) = self.get_mode_config(&mode) {
            // Update task priorities based on mode
            self.update_task_priorities(&mode_config.default_priorities).await?;
            
            // Re-sort the task queue based on new priorities
            self.resort_task_queue().await?;
            
            // Log the mode change
            self.base.log_event(
                EventType::StateChange, 
                &format!("Switched to {} mode", self.get_mode_name(&mode)), 
                Some(json!({
                    "previous_mode": self.get_mode_name(&current_mode),
                    "new_mode": self.get_mode_name(&mode),
                    "priority_changes": mode_config.default_priorities,
                }))
            ).await;
        }
        
        Ok(())
    }
    
    /// Add a task to the queue
    pub async fn enqueue_task(&self, task: PrioritizedTask) -> Result<(), ComponentError> {
        let mut queue = self.task_queue.lock().await;
        
        // Add to queue
        queue.push_back(task.clone());
        
        // Log the task addition
        debug!("Enqueued task: {} (priority {})", task.task.description, task.priority);
        
        // Update stats
        let mut stats = self.stats.write().await;
        stats.avg_queue_size = ((stats.avg_queue_size * stats.total_tasks_processed as f64) + queue.len() as f64) / 
            (stats.total_tasks_processed + 1) as f64;
        
        // Resort queue by priority
        self.resort_task_queue().await?;
        
        Ok(())
    }
    
    /// Schedule a task for future execution
    pub async fn schedule_task(&self, task: PrioritizedTask, when: DateTime<Utc>, recurrence: Option<RecurrencePattern>) -> Result<(), ComponentError> {
        let scheduled_task = ScheduledTask {
            task,
            scheduled_time: when,
            recurrence,
            execution_count: 0,
        };
        
        let mut scheduled = self.scheduled_tasks.lock().await;
        scheduled.push(scheduled_task.clone());
        
        debug!("Scheduled task: {} for {}", scheduled_task.task.task.description, when);
        
        // Sort by scheduled time
        scheduled.sort_by(|a, b| a.scheduled_time.cmp(&b.scheduled_time));
        
        Ok(())
    }
    
    /// Get the next task to process
    pub async fn get_next_task(&self) -> Option<PrioritizedTask> {
        let mut queue = self.task_queue.lock().await;
        
        if queue.is_empty() {
            return None;
        }
        
        // In a real implementation, we'd check resource availability here
        queue.pop_front()
    }
    
    /// Start processing a task
    pub async fn start_task(&self, task: &PrioritizedTask) -> Result<(), ComponentError> {
        let task_id = &task.task.id;
        let now = Utc::now();
        
        // Calculate expected completion time
        let expected_completion = now + Duration::seconds(task.estimated_duration as i64);
        
        // Allocate resources
        let allocated_resources = self.allocate_resources(task).await?;
        
        // Create running task info
        let task_info = RunningTaskInfo {
            task_id: task_id.clone(),
            description: task.task.description.clone(),
            category: task.category.clone(),
            started_at: now,
            expected_completion,
            allocated_resources,
            progress: 0.0,
            interruptible: task.interruptible,
            state_snapshot: None,
            last_snapshot_time: now,
        };
        
        // Add to running tasks
        let mut running = self.running_tasks.lock().await;
        running.insert(task_id.clone(), task_info.clone());
        
        debug!("Started task: {} ({})", task_id, task.task.description);
        
        // Log the task start
        self.base.log_event(
            EventType::TaskProcessing, 
            &format!("Started task {}", task_id), 
            Some(json!({
                "description": task.task.description,
                "category": task.category,
                "priority": task.priority,
                "resources": allocated_resources,
            }))
        ).await;
        
        Ok(())
    }
    
    /// Complete a task
    pub async fn complete_task(&self, task_id: &str, status: TaskStatus, result: Option<serde_json::Value>) -> Result<(), ComponentError> {
        let now = Utc::now();
        
        // Get the running task info
        let task_info = {
            let mut running = self.running_tasks.lock().await;
            if let Some(info) = running.remove(task_id) {
                info
            } else {
                return Err(ComponentError::ValidationError(format!(
                    "Task not found in running tasks: {}", task_id
                )));
            }
        };
        
        // Release resources
        self.release_resources(&task_info.allocated_resources).await?;
        
        // Create history entry
        let history_entry = TaskHistoryEntry {
            task_id: task_id.to_string(),
            description: task_info.description.clone(),
            category: task_info.category.clone(),
            priority: 0, // Would be filled from original task in a real implementation
            queued_at: task_info.started_at - Duration::seconds(10), // Approximation
            started_at: task_info.started_at,
            completed_at: now,
            status: status.clone(),
            resources_used: task_info.allocated_resources.clone(),
            result_summary: result.as_ref().map(|r| r.to_string()),
        };
        
        // Add to history
        let mut history = self.task_history.lock().await;
        history.push_back(history_entry);
        
        // Trim history if needed
        if history.len() > 1000 {
            history.pop_front();
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.total_tasks_processed += 1;
            
            // Update category stats
            let category_count = stats.tasks_by_category.entry(task_info.category.clone()).or_insert(0);
            *category_count += 1;
            
            // Update status stats
            let status_name = format!("{:?}", status);
            let status_count = stats.tasks_by_status.entry(status_name).or_insert(0);
            *status_count += 1;
            
            // Update waiting time stats
            let waiting_time = (task_info.started_at - history_entry.queued_at).num_seconds() as f64;
            stats.avg_waiting_time_secs = (
                (stats.avg_waiting_time_secs * (stats.total_tasks_processed - 1) as f64) + waiting_time
            ) / stats.total_tasks_processed as f64;
        }
        
        debug!("Completed task: {} with status {:?}", task_id, status);
        
        // Log the task completion
        self.base.log_event(
            EventType::TaskProcessing, 
            &format!("Completed task {}", task_id), 
            Some(json!({
                "description": task_info.description,
                "status": format!("{:?}", status),
                "duration_secs": (now - task_info.started_at).num_seconds(),
            }))
        ).await;
        
        Ok(())
    }
    
    /// Update the progress of a running task
    pub async fn update_task_progress(&self, task_id: &str, progress: f64) -> Result<(), ComponentError> {
        let mut running = self.running_tasks.lock().await;
        
        if let Some(task) = running.get_mut(task_id) {
            task.progress = progress.max(0.0).min(1.0);
            
            // If this is a significant progress point, take a snapshot
            if progress > 0.25 && task.state_snapshot.is_none() {
                task.state_snapshot = Some(json!({
                    "progress": progress,
                    "timestamp": Utc::now(),
                }));
                task.last_snapshot_time = Utc::now();
            }
            
            Ok(())
        } else {
            Err(ComponentError::ValidationError(format!(
                "Task not found in running tasks: {}", task_id
            )))
        }
    }
    
    /// Take a snapshot of a running task for recovery
    pub async fn snapshot_task(&self, task_id: &str, state: serde_json::Value) -> Result<(), ComponentError> {
        let mut running = self.running_tasks.lock().await;
        
        if let Some(task) = running.get_mut(task_id) {
            task.state_snapshot = Some(state);
            task.last_snapshot_time = Utc::now();
            Ok(())
        } else {
            Err(ComponentError::ValidationError(format!(
                "Task not found in running tasks: {}", task_id
            )))
        }
    }
    
    /// Handle an interruption (e.g., shutdown)
    pub async fn handle_interruption(&self) -> Result<(), ComponentError> {
        info!("Handling system interruption");
        
        let mut recovery_data = RecoveryData {
            last_mode: Some(self.mode.read().await.clone()),
            interrupted_tasks: Vec::new(),
            pending_tasks: Vec::new(),
            interruption_time: Some(Utc::now()),
            recovery_state: RecoveryState::Needed,
            interrupted_goals: Vec::new(),
        };
        
        // Capture running tasks
        {
            let running = self.running_tasks.lock().await;
            recovery_data.interrupted_tasks = running.values().cloned().collect();
        }
        
        // Capture high-priority pending tasks
        {
            let queue = self.task_queue.lock().await;
            recovery_data.pending_tasks = queue.iter()
                .filter(|task| task.priority >= 8) // Only high-priority tasks
                .cloned()
                .collect();
        }
        
        // Capture active goals
        {
            let goals = self.goals.read().await;
            recovery_data.interrupted_goals = goals.clone();
        }
        
        // Save recovery data
        *self.recovery_data.write().await = recovery_data;
        
        // Log the interruption
        self.base.log_event(
            EventType::StateChange, 
            "System interrupted", 
            Some(json!({
                "interrupted_tasks": self.running_tasks.lock().await.len(),
                "high_priority_pending": recovery_data.pending_tasks.len(),
                "time": Utc::now(),
            }))
        ).await;
        
        Ok(())
    }
    
    /// Recover from an interruption
    pub async fn recover_from_interruption(&self) -> Result<(), ComponentError> {
        let recovery_data = self.recovery_data.read().await.clone();
        
        if recovery_data.recovery_state != RecoveryState::Needed {
            debug!("No recovery needed or already completed");
            return Ok(());
        }
        
        info!("Recovering from system interruption");
        
        // Mark recovery as in progress
        {
            let mut data = self.recovery_data.write().await;
            data.recovery_state = RecoveryState::InProgress;
        }
        
        // Restore last mode
        if let Some(mode) = &recovery_data.last_mode {
            self.switch_mode(mode.clone()).await?;
        }
        
        // Re-queue interrupted high-priority tasks
        for task in &recovery_data.pending_tasks {
            self.enqueue_task(task.clone()).await?;
        }
        
        // For each interrupted task, create a recovery task
        for interrupted in &recovery_data.interrupted_tasks {
            if interrupted.interruptible {
                // For interruptible tasks, we can create a continuation task
                // with the last snapshot state if available
                if let Some(snapshot) = &interrupted.state_snapshot {
                    // Create a new task with the snapshot state as a parameter
                    let recovery_task = ComponentTask {
                        id: uuid::Uuid::new_v4().to_string(),
                        description: format!("Recovery for: {}", interrupted.description),
                        parameters: json!({
                            "operation": "recover",
                            "original_task_id": interrupted.task_id,
                            "state_snapshot": snapshot,
                            "progress": interrupted.progress,
                        }),
                        priority: 10, // High priority for recovery
                        deadline: None,
                        created_at: Utc::now(),
                    };
                    
                    let prioritized = PrioritizedTask {
                        task: recovery_task,
                        category: interrupted.category.clone(),
                        priority: 10,
                        required_resources: interrupted.allocated_resources.clone(),
                        estimated_duration: 60.0, // Estimate for recovery
                        is_system_task: true,
                        interruptible: false, // Don't interrupt recovery
                        queued_at: Utc::now(),
                    };
                    
                    self.enqueue_task(prioritized).await?;
                }
            }
        }
        
        // Mark recovery as completed
        {
            let mut data = self.recovery_data.write().await;
            data.recovery_state = RecoveryState::Completed;
        }
        
        // Log the recovery
        self.base.log_event(
            EventType::StateChange, 
            "System recovered from interruption", 
            Some(json!({
                "recovered_tasks": recovery_data.interrupted_tasks.len(),
                "time": Utc::now(),
            }))
        ).await;
        
        Ok(())
    }
    
    /// Create a new goal
    pub async fn create_goal(&self, description: &str, priority: u8) -> Result<String, ComponentError> {
        let id = uuid::Uuid::new_v4().to_string();
        
        let goal = Goal {
            id: id.clone(),
            description: description.to_string(),
            priority,
            created_at: Utc::now(),
            deadline: None,
            targets: Vec::new(),
            status: GoalStatus::Pending,
            related_tasks: Vec::new(),
            parent_goal: None,
            child_goals: Vec::new(),
        };
        
        // Add to goals
        {
            let mut goals = self.goals.write().await;
            goals.push(goal.clone());
            
            // Sort by priority
            goals.sort_by(|a, b| b.priority.cmp(&a.priority));
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.goal_stats.total_goals += 1;
            let status_count = stats.goal_stats.goals_by_status.entry("Pending".to_string()).or_insert(0);
            *status_count += 1;
        }
        
        debug!("Created goal: {} ({})", id, description);
        
        // Log goal creation
        self.base.log_event(
            EventType::Info, 
            &format!("Created goal: {}", description), 
            Some(json!({
                "id": id,
                "priority": priority,
            }))
        ).await;
        
        Ok(id)
    }
    
    /// Add a target to a goal
    pub async fn add_goal_target(&self, goal_id: &str, description: &str, criteria: serde_json::Value) -> Result<String, ComponentError> {
        let id = uuid::Uuid::new_v4().to_string();
        
        let target = GoalTarget {
            id: id.clone(),
            description: description.to_string(),
            criteria,
            progress: 0.0,
            status: GoalStatus::Pending,
        };
        
        // Add to goal
        {
            let mut goals = self.goals.write().await;
            
            if let Some(goal) = goals.iter_mut().find(|g| g.id == goal_id) {
                goal.targets.push(target.clone());
                
                // Update goal status if needed
                if goal.status == GoalStatus::Pending && goal.targets.len() > 0 {
                    goal.status = GoalStatus::InProgress;
                    
                    // Update stats
                    let mut stats = self.stats.write().await;
                    let pending_count = stats.goal_stats.goals_by_status.entry("Pending".to_string()).or_insert(0);
                    *pending_count -= 1;
                    let in_progress_count = stats.goal_stats.goals_by_status.entry("InProgress".to_string()).or_insert(0);
                    *in_progress_count += 1;
                }
                
                Ok(id)
            } else {
                Err(ComponentError::ValidationError(format!(
                    "Goal not found: {}", goal_id
                )))
            }
        }
    }
    
    /// Update goal target progress
    pub async fn update_target_progress(&self, goal_id: &str, target_id: &str, progress: f64) -> Result<(), ComponentError> {
        let mut goals = self.goals.write().await;
        
        if let Some(goal) = goals.iter_mut().find(|g| g.id == goal_id) {
            if let Some(target) = goal.targets.iter_mut().find(|t| t.id == target_id) {
                target.progress = progress.max(0.0).min(1.0);
                
                // Update target status if needed
                if progress >= 1.0 && target.status != GoalStatus::Completed {
                    target.status = GoalStatus::Completed;
                }
                
                // Check if all targets are completed
                let all_completed = goal.targets.iter().all(|t| t.status == GoalStatus::Completed);
                
                if all_completed && goal.status != GoalStatus::Completed {
                    // Update goal status
                    let old_status = goal.status.clone();
                    goal.status = GoalStatus::Completed;
                    
                    // Update stats
                    let mut stats = self.stats.write().await;
                    let old_count = stats.goal_stats.goals_by_status.entry(format!("{:?}", old_status)).or_insert(0);
                    *old_count = old_count.saturating_sub(1);
                    let completed_count = stats.goal_stats.goals_by_status.entry("Completed".to_string()).or_insert(0);
                    *completed_count += 1;
                    
                    // Update average completion time
                    let duration_secs = (Utc::now() - goal.created_at).num_seconds() as f64;
                    let completed_goals = *completed_count as f64;
                    stats.goal_stats.avg_completion_time_secs = (
                        stats.goal_stats.avg_completion_time_secs * (completed_goals - 1.0) + duration_secs
                    ) / completed_goals;
                    
                    // Update success rate
                    let total = stats.goal_stats.total_goals as f64;
                    if total > 0.0 {
                        stats.goal_stats.success_rate = completed_goals / total;
                    }
                    
                    // Log goal completion
                    self.base.log_event(
                        EventType::Info, 
                        &format!("Completed goal: {}", goal.description), 
                        Some(json!({
                            "id": goal_id,
                            "duration_secs": duration_secs,
                        }))
                    ).await;
                }
                
                Ok(())
            } else {
                Err(ComponentError::ValidationError(format!(
                    "Target not found: {}", target_id
                )))
            }
        } else {
            Err(ComponentError::ValidationError(format!(
                "Goal not found: {}", goal_id
            )))
        }
    }
    
    /// Run autonomous operations (mode switching, task generation, etc.)
    pub async fn run_autonomous_operations(&self) -> Result<(), ComponentError> {
        if !self.autonomous_settings.enabled {
            return Ok(());
        }
        
        debug!("Running autonomous operations");
        
        // Check if it's time to switch modes
        self.check_mode_switching().await?;
        
        // Check for scheduled tasks
        self.check_scheduled_tasks().await?;
        
        // Generate tasks if needed
        if self.autonomous_settings.task_generation.enabled {
            self.generate_tasks().await?;
        }
        
        // Generate goals if needed
        if self.autonomous_settings.goal_generation.enabled {
            self.generate_goals().await?;
        }
        
        Ok(())
    }
    
    /// Check if scheduled tasks need to be executed
    async fn check_scheduled_tasks(&self) -> Result<(), ComponentError> {
        let now = Utc::now();
        let mut to_execute = Vec::new();
        let mut to_reschedule = Vec::new();
        
        // Find scheduled tasks that are due
        {
            let mut scheduled = self.scheduled_tasks.lock().await;
            let mut idx = 0;
            
            while idx < scheduled.len() {
                if scheduled[idx].scheduled_time <= now {
                    // Task is due
                    let task = scheduled.remove(idx);
                    
                    // If task has recurrence, add to reschedule list
                    if let Some(recurrence) = &task.recurrence {
                        to_reschedule.push((task.clone(), recurrence.clone()));
                    }
                    
                    to_execute.push(task.task.clone());
                } else {
                    // Skip to next task
                    idx += 1;
                }
            }
        }
        
        // Execute due tasks
        for task in to_execute {
            self.enqueue_task(task).await?;
        }
        
        // Reschedule recurring tasks
        for (task, recurrence) in to_reschedule {
            let next_time = self.calculate_next_occurrence(task.scheduled_time, &recurrence);
            
            let mut new_task = task;
            new_task.scheduled_time = next_time;
            new_task.execution_count += 1;
            
            let mut scheduled = self.scheduled_tasks.lock().await;
            scheduled.push(new_task);
            
            // Sort by scheduled time
            scheduled.sort_by(|a, b| a.scheduled_time.cmp(&b.scheduled_time));
        }
        
        Ok(())
    }
    
    /// Calculate the next occurrence for a recurring task
    fn calculate_next_occurrence(&self, last_time: DateTime<Utc>, recurrence: &RecurrencePattern) -> DateTime<Utc> {
        match recurrence {
            RecurrencePattern::EverySeconds(secs) => last_time + Duration::seconds(*secs as i64),
            RecurrencePattern::EveryMinutes(mins) => last_time + Duration::minutes(*mins as i64),
            RecurrencePattern::EveryHours(hours) => last_time + Duration::hours(*hours as i64),
            RecurrencePattern::EveryDays(days) => last_time + Duration::days(*days as i64),
            RecurrencePattern::WeeklyOnDays(_days) => {
                // This would require more complex logic to find the next matching day of week
                // For the MVP, we'll just advance by 7 days
                last_time + Duration::days(7)
            },
            RecurrencePattern::MonthlyOnDays(_days) => {
                // This would require more complex logic to find the next matching day of month
                // For the MVP, we'll just advance by 30 days
                last_time + Duration::days(30)
            },
            RecurrencePattern::Custom(_) => {
                // Custom recurrence would need special handling
                // For the MVP, we'll just advance by 1 day
                last_time + Duration::days(1)
            },
        }
    }
    
    /// Check if it's time to switch modes
    async fn check_mode_switching(&self) -> Result<(), ComponentError> {
        let current_mode = self.mode.read().await.clone();
        
        match &self.autonomous_settings.mode_switching {
            ModeSwitchingStrategy::TimeBased { schedule } => {
                // In a real system, we'd track the time spent in each mode
                // and switch based on the schedule
                // For the MVP, we'll just check if we've been in this mode long enough
                
                // Find the next mode in the schedule
                if let Some(idx) = schedule.iter().position(|(mode, _)| mode == &current_mode) {
                    // Get the next mode in the rotation
                    let next_idx = (idx + 1) % schedule.len();
                    let (next_mode, _) = &schedule[next_idx];
                    
                    // For demonstration, randomly decide to switch
                    if rand::random::<f64>() < 0.1 {
                        // Switch to the next mode
                        self.switch_mode(next_mode.clone()).await?;
                    }
                }
            },
            ModeSwitchingStrategy::StateBased { conditions: _ } => {
                // This would evaluate system state to determine if a mode switch is needed
                // Not implemented in the MVP
            },
            ModeSwitchingStrategy::ResourceBased => {
                // Check resource usage and switch if certain thresholds are crossed
                // Not fully implemented in the MVP
                
                // For demonstration, check if CPU usage is high
                let high_cpu = self.resource_allocation.allocated_resources.get("cpu")
                    .map(|&usage| usage > 0.8)
                    .unwrap_or(false);
                
                if high_cpu && current_mode != OperationalMode::ProblemSolving {
                    // Switch to problem solving to focus resources
                    self.switch_mode(OperationalMode::ProblemSolving).await?;
                }
            },
            ModeSwitchingStrategy::GoalBased => {
                // Switch based on active goals
                // Not implemented in the MVP
            },
            ModeSwitchingStrategy::Manual => {
                // No automatic switching
            },
        }
        
        Ok(())
    }
    
    /// Generate tasks based on current system state
    async fn generate_tasks(&self) -> Result<(), ComponentError> {
        // In a real system, this would analyze the current state and generate
        // tasks based on needs, patterns, goals, etc.
        // For the MVP, we'll just generate a simple exploration task
        
        // Only generate a task 10% of the time for demonstration
        if rand::random::<f64>() < 0.1 {
            let task = ComponentTask {
                id: uuid::Uuid::new_v4().to_string(),
                description: "Auto-generated exploration task".to_string(),
                parameters: json!({
                    "operation": "explore",
                    "depth": 3,
                    "breadth": 5,
                }),
                priority: 5,
                deadline: None,
                created_at: Utc::now(),
            };
            
            let prioritized = PrioritizedTask {
                task,
                category: "exploration".to_string(),
                priority: 5,
                required_resources: HashMap::from([
                    ("cpu".to_string(), 0.2),
                    ("memory".to_string(), 0.3),
                    ("attention".to_string(), 0.4),
                    ("creativity".to_string(), 0.6),
                ]),
                estimated_duration: 120.0,
                is_system_task: true,
                interruptible: true,
                queued_at: Utc::now(),
            };
            
            self.enqueue_task(prioritized).await?;
        }
        
        Ok(())
    }
    
    /// Generate goals based on current system state
    async fn generate_goals(&self) -> Result<(), ComponentError> {
        // Check if we're at max goals
        let goals = self.goals.read().await;
        let active_goals = goals.iter()
            .filter(|g| g.status == GoalStatus::Pending || g.status == GoalStatus::InProgress)
            .count();
        
        if active_goals >= self.autonomous_settings.goal_generation.max_active_goals {
            return Ok(());
        }
        
        drop(goals); // Release the read lock
        
        // Only generate a goal 5% of the time for demonstration
        if rand::random::<f64>() < 0.05 {
            let goal_desc = match self.mode.read().await.clone() {
                OperationalMode::ProblemSolving => "Improve problem-solving efficiency",
                OperationalMode::Exploration => "Discover new knowledge patterns",
                OperationalMode::Consolidation => "Optimize memory organization",
                OperationalMode::Planning => "Develop long-term operational strategy",
                OperationalMode::Custom(_) => "Achieve custom operational objective",
            };
            
            let goal_id = self.create_goal(goal_desc, 7).await?;
            
            // Add a target to the goal
            self.add_goal_target(
                &goal_id,
                "Reach operational milestone",
                json!({
                    "measure": "efficiency",
                    "target_value": 0.85,
                    "timeframe_days": 1
                })
            ).await?;
        }
        
        Ok(())
    }
    
    /// Allocate resources for a task
    async fn allocate_resources(&self, task: &PrioritizedTask) -> Result<HashMap<String, f64>, ComponentError> {
        let mut allocated = HashMap::new();
        let mut resource_allocation = self.resource_allocation.clone();
        
        // For each resource required by the task
        for (resource, amount) in &task.required_resources {
            // Check if we have this resource
            if let Some(available) = resource_allocation.total_resources.get(resource) {
                // Check if we have enough available
                let currently_allocated = *resource_allocation.allocated_resources.get(resource).unwrap_or(&0.0);
                let remaining = available - currently_allocated;
                
                if remaining >= *amount {
                    // Allocate the resource
                    let current = resource_allocation.allocated_resources.entry(resource.clone()).or_insert(0.0);
                    *current += amount;
                    allocated.insert(resource.clone(), *amount);
                } else {
                    // Not enough available, scale down the allocation
                    let scaled = remaining.max(0.0);
                    let current = resource_allocation.allocated_resources.entry(resource.clone()).or_insert(0.0);
                    *current += scaled;
                    allocated.insert(resource.clone(), scaled);
                    
                    warn!("Insufficient {} resource. Requested: {}, Allocated: {}", resource, amount, scaled);
                }
            } else {
                // Resource not recognized
                warn!("Unknown resource requested: {}", resource);
            }
        }
        
        // Update the resource allocation
        self.resource_allocation = resource_allocation;
        
        Ok(allocated)
    }
    
    /// Release allocated resources
    async fn release_resources(&self, resources: &HashMap<String, f64>) -> Result<(), ComponentError> {
        let mut resource_allocation = self.resource_allocation.clone();
        
        for (resource, amount) in resources {
            if let Some(allocated) = resource_allocation.allocated_resources.get_mut(resource) {
                // Release the resource
                *allocated -= amount;
                
                // Ensure we don't go negative
                if *allocated < 0.0 {
                    *allocated = 0.0;
                }
            }
        }
        
        // Update the resource allocation
        self.resource_allocation = resource_allocation;
        
        Ok(())
    }
    
    /// Capture the state of the current mode for later restoration
    async fn capture_mode_state(&self, mode: &OperationalMode) -> Result<(), ComponentError> {
        // In a real implementation, this would save state specific to the mode
        // For the MVP, we'll just log the mode change
        debug!("Capturing state for {} mode", self.get_mode_name(mode));
        Ok(())
    }
    
    /// Update task priorities based on mode
    async fn update_task_priorities(&self, priorities: &HashMap<String, u8>) -> Result<(), ComponentError> {
        let mut queue = self.task_queue.lock().await;
        
        // Update priorities for queued tasks
        for task in queue.iter_mut() {
            if let Some(&priority) = priorities.get(&task.category) {
                // Blend the original priority with the mode-based priority
                // Original priority has 30% weight, mode-based has 70%
                let new_priority = ((task.priority as f64 * 0.3) + (priority as f64 * 0.7)) as u8;
                task.priority = new_priority;
            }
        }
        
        Ok(())
    }
    
    /// Resort the task queue based on priorities
    async fn resort_task_queue(&self) -> Result<(), ComponentError> {
        let mut queue = self.task_queue.lock().await;
        
        // Convert to vector for sorting
        let mut tasks: Vec<_> = queue.drain(..).collect();
        
        // Sort by priority (higher first)
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // Put back in the queue
        for task in tasks {
            queue.push_back(task);
        }
        
        Ok(())
    }
    
    /// Get the name of an operational mode
    fn get_mode_name(&self, mode: &OperationalMode) -> String {
        match mode {
            OperationalMode::ProblemSolving => "Problem Solving".to_string(),
            OperationalMode::Exploration => "Exploration".to_string(),
            OperationalMode::Consolidation => "Consolidation".to_string(),
            OperationalMode::Planning => "Planning".to_string(),
            OperationalMode::Custom(name) => format!("Custom ({})", name),
        }
    }
    
    /// Get the configuration for a mode
    fn get_mode_config(&self, mode: &OperationalMode) -> Option<OperationalModeConfig> {
        self.available_modes.iter()
            .find(|m| m.mode == *mode)
            .cloned()
    }
    
    /// Get operational stats
    pub async fn get_stats(&self) -> OperationalStats {
        self.stats.read().await.clone()
    }
    
    /// Get current goals
    pub async fn get_goals(&self) -> Vec<Goal> {
        self.goals.read().await.clone()
    }
    
    /// Get task history
    pub async fn get_task_history(&self, limit: usize) -> Vec<TaskHistoryEntry> {
        let history = self.task_history.lock().await;
        let start = if history.len() > limit {
            history.len() - limit
        } else {
            0
        };
        
        history.range(start..).cloned().collect()
    }
}

impl Clone for OperationalStateInstance {
    fn clone(&self) -> Self {
        // Implement clone for testing purposes
        // In a real system, we'd properly clone all the values
        Self::new()
    }
}

#[async_trait]
impl Component for OperationalStateInstance {
    fn id(&self) -> &str {
        &self.base.id
    }
    
    fn component_type(&self) -> &str {
        &self.base.component_type
    }
    
    fn status(&self) -> ComponentStatus {
        self.base.status.clone()
    }
    
    async fn initialize(&mut self, config: ComponentConfig) -> Result<(), ComponentError> {
        info!("Initializing OperationalStateInstance with config: {}", config.id);
        self.base.config = Some(config.clone());
        
        // Extract configuration parameters
        if let Some(mode) = config.parameters.get("default_mode").and_then(|v| v.as_str()) {
            match mode {
                "problem_solving" => *self.mode.write().await = OperationalMode::ProblemSolving,
                "exploration" => *self.mode.write().await = OperationalMode::Exploration,
                "consolidation" => *self.mode.write().await = OperationalMode::Consolidation,
                "planning" => *self.mode.write().await = OperationalMode::Planning,
                _ => *self.mode.write().await = OperationalMode::ProblemSolving,
            }
        }
        
        // Configure resources if specified
        if let Some(resources) = config.parameters.get("resources").and_then(|v| v.as_object()) {
            for (resource, value) in resources {
                if let Some(amount) = value.as_f64() {
                    self.resource_allocation.total_resources.insert(resource.clone(), amount);
                }
            }
        }
        
        // Configure autonomous settings if specified
        if let Some(autonomous) = config.parameters.get("autonomous") {
            if let Some(enabled) = autonomous.get("enabled").and_then(|v| v.as_bool()) {
                self.autonomous_settings.enabled = enabled;
            }
            
            if let Some(min_time) = autonomous.get("min_time_in_mode_secs").and_then(|v| v.as_u64()) {
                self.autonomous_settings.min_time_in_mode_secs = min_time;
            }
        }
        
        // Initialize recovery state
        let recovery_state = RecoveryState::None;
        self.recovery_data.write().await.recovery_state = recovery_state;
        
        self.base.status = ComponentStatus::Initialized;
        self.base.log_event(
            EventType::Initialization, 
            "OperationalStateInstance initialized", 
            Some(json!({
                "initial_mode": self.get_mode_name(&self.mode.read().await),
                "autonomous_enabled": self.autonomous_settings.enabled,
            }))
        ).await;
        
        Ok(())
    }
    
    async fn start(&mut self) -> Result<(), ComponentError> {
        info!("Starting OperationalStateInstance");
        self.base.status = ComponentStatus::Running;
        
        // Check if recovery is needed
        if self.recovery_data.read().await.recovery_state == RecoveryState::Needed {
            self.recover_from_interruption().await?;
        }
        
        // Start task processing loop
        let instance = self.clone();
        tokio::spawn(async move {
            instance.task_processing_loop().await;
        });
        
        // Start autonomous operations loop
        let instance = self.clone();
        tokio::spawn(async move {
            instance.autonomous_operations_loop().await;
        });
        
        // Start scheduled task checker loop
        let instance = self.clone();
        tokio::spawn(async move {
            instance.scheduled_task_checker_loop().await;
        });
        
        self.base.log_event(
            EventType::StateChange, 
            "OperationalStateInstance started", 
            Some(json!({
                "mode": self.get_mode_name(&self.mode.read().await),
                "recovery_performed": self.recovery_data.read().await.recovery_state == RecoveryState::Completed,
            }))
        ).await;
        
        Ok(())
    }
    
    async fn pause(&mut self) -> Result<(), ComponentError> {
        info!("Pausing OperationalStateInstance");
        self.base.status = ComponentStatus::Paused;
        
        self.base.log_event(
            EventType::StateChange, 
            "OperationalStateInstance paused", 
            None
        ).await;
        
        Ok(())
    }
    
    async fn resume(&mut self) -> Result<(), ComponentError> {
        info!("Resuming OperationalStateInstance");
        self.base.status = ComponentStatus::Running;
        
        self.base.log_event(
            EventType::StateChange, 
            "OperationalStateInstance resumed", 
            None
        ).await;
        
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), ComponentError> {
        info!("Shutting down OperationalStateInstance");
        self.base.status = ComponentStatus::ShuttingDown;
        
        // Handle interruption for graceful shutdown
        self.handle_interruption().await?;
        
        self.base.log_event(
            EventType::StateChange, 
            "OperationalStateInstance shutting down", 
            None
        ).await;
        
        Ok(())
    }
    
    async fn process_task(&mut self, task: ComponentTask) -> Result<serde_json::Value, ComponentError> {
        debug!("Processing task: {}", task.id);
        self.base.log_event(
            EventType::TaskProcessing, 
            &format!("Processing task {}", task.id), 
            Some(json!({"description": task.description}))
        ).await;
        
        // Determine the operation type from task parameters
        let operation = extract_task_param::<String>(&task, "operation")
            .unwrap_or_else(|_| "default".to_string());
        
        let (result, duration_ms) = match operation.as_str() {
            "switch_mode" => {
                let mode_str = extract_task_param::<String>(&task, "mode")?;
                
                let mode = match mode_str.as_str() {
                    "problem_solving" => OperationalMode::ProblemSolving,
                    "exploration" => OperationalMode::Exploration,
                    "consolidation" => OperationalMode::Consolidation,
                    "planning" => OperationalMode::Planning,
                    _ => OperationalMode::Custom(mode_str),
                };
                
                let result_future = self.switch_mode(mode);
                let (result, duration) = measure_execution_time(result_future).await;
                
                (
                    result.map(|_| json!({
                        "success": true,
                        "operation": "switch_mode",
                        "new_mode": self.get_mode_name(&mode),
                    })),
                    duration
                )
            },
            "create_goal" => {
                let description = extract_task_param::<String>(&task, "description")?;
                let priority = extract_task_param::<u8>(&task, "priority").unwrap_or(5);
                
                let result_future = self.create_goal(&description, priority);
                let (result, duration) = measure_execution_time(result_future).await;
                
                (
                    result.map(|id| json!({
                        "success": true,
                        "operation": "create_goal",
                        "goal_id": id,
                    })),
                    duration
                )
            },
            "add_goal_target" => {
                let goal_id = extract_task_param::<String>(&task, "goal_id")?;
                let description = extract_task_param::<String>(&task, "description")?;
                let criteria = extract_task_param::<serde_json::Value>(&task, "criteria")
                    .unwrap_or_else(|_| json!({}));
                
                let result_future = self.add_goal_target(&goal_id, &description, criteria);
                let (result, duration) = measure_execution_time(result_future).await;
                
                (
                    result.map(|id| json!({
                        "success": true,
                        "operation": "add_goal_target",
                        "target_id": id,
                    })),
                    duration
                )
            },
            "update_target" => {
                let goal_id = extract_task_param::<String>(&task, "goal_id")?;
                let target_id = extract_task_param::<String>(&task, "target_id")?;
                let progress = extract_task_param::<f64>(&task, "progress")?;
                
                let result_future = self.update_target_progress(&goal_id, &target_id, progress);
                let (result, duration) = measure_execution_time(result_future).await;
                
                (
                    result.map(|_| json!({
                        "success": true,
                        "operation": "update_target",
                        "goal_id": goal_id,
                        "target_id": target_id,
                        "progress": progress,
                    })),
                    duration
                )
            },
            "schedule_task" => {
                // Extract task details
                let subtask = extract_task_param::<ComponentTask>(&task, "task")?;
                let category = extract_task_param::<String>(&task, "category").unwrap_or("default".to_string());
                let priority = extract_task_param::<u8>(&task, "priority").unwrap_or(5);
                let required_resources = extract_task_param::<HashMap<String, f64>>(&task, "resources")
                    .unwrap_or_else(|_| HashMap::new());
                let estimated_duration = extract_task_param::<f64>(&task, "duration").unwrap_or(60.0);
                let when_str = extract_task_param::<String>(&task, "when")?;
                let recurrence_str = task.parameters.get("recurrence").and_then(|v| v.as_str());
                
                // Parse the scheduled time
                let when = chrono::DateTime::parse_from_rfc3339(&when_str)
                    .map_err(|e| ComponentError::ValidationError(format!("Invalid date format: {}", e)))?;
                let when_utc = when.with_timezone(&Utc);
                
                // Parse recurrence if any
                let recurrence = if let Some(rec_str) = recurrence_str {
                    match rec_str {
                        "hourly" => Some(RecurrencePattern::EveryHours(1)),
                        "daily" => Some(RecurrencePattern::EveryDays(1)),
                        "weekly" => Some(RecurrencePattern::WeeklyOnDays(vec![0])),
                        "monthly" => Some(RecurrencePattern::MonthlyOnDays(vec![1])),
                        _ => None,
                    }
                } else {
                    None
                };
                
                // Create prioritized task
                let prioritized = PrioritizedTask {
                    task: subtask,
                    category,
                    priority,
                    required_resources,
                    estimated_duration,
                    is_system_task: false,
                    interruptible: true,
                    queued_at: Utc::now(),
                };
                
                let result_future = self.schedule_task(prioritized, when_utc, recurrence);
                let (result, duration) = measure_execution_time(result_future).await;
                
                (
                    result.map(|_| json!({
                        "success": true,
                        "operation": "schedule_task",
                        "scheduled_time": when_utc,
                    })),
                    duration
                )
            },
            "get_stats" => {
                let stats_future = self.get_stats();
                
                let (stats, duration) = measure_execution_time(stats_future).await;
                (Ok(serde_json::to_value(stats).unwrap()), duration)
            },
            "get_goals" => {
                let goals_future = self.get_goals();
                
                let (goals, duration) = measure_execution_time(goals_future).await;
                (Ok(serde_json::to_value(goals).unwrap()), duration)
            },
            "get_history" => {
                let limit = extract_task_param::<usize>(&task, "limit").unwrap_or(100);
                let history_future = self.get_task_history(limit);
                
                let (history, duration) = measure_execution_time(history_future).await;
                (Ok(serde_json::to_value(history).unwrap()), duration)
            },
            _ => {
                // Default operation: queue a task for processing
                let prioritized = PrioritizedTask {
                    task: task.clone(),
                    category: extract_task_param::<String>(&task, "category").unwrap_or("default".to_string()),
                    priority: task.priority,
                    required_resources: extract_task_param::<HashMap<String, f64>>(&task, "resources")
                        .unwrap_or_else(|_| HashMap::new()),
                    estimated_duration: extract_task_param::<f64>(&task, "duration").unwrap_or(60.0),
                    is_system_task: false,
                    interruptible: true,
                    queued_at: Utc::now(),
                };
                
                let result_future = self.enqueue_task(prioritized);
                let (result, duration) = measure_execution_time(result_future).await;
                
                (
                    result.map(|_| json!({
                        "success": true,
                        "operation": "enqueue_task",
                        "task_id": task.id,
                    })),
                    duration
                )
            }
        };
        
        // Record task processing in metrics
        self.base.record_task_processing(result.is_ok(), duration_ms);
        
        // Return result
        match result {
            Ok(value) => Ok(value),
            Err(e) => Err(e),
        }
    }
    
    async fn handle_message(&mut self, message: ComponentMessage) -> Result<(), ComponentError> {
        debug!("Handling message: {}", message.id);
        self.base.log_event(
            EventType::Info, 
            &format!("Received message {}", message.id), 
            Some(json!({"sender": message.sender, "type": message.message_type}))
        ).await;
        
        // Handle different message types
        match message.message_type.as_str() {
            "switch_mode" => {
                if let Some(mode) = message.payload.get("mode").and_then(|v| v.as_str()) {
                    match mode {
                        "problem_solving" => self.switch_mode(OperationalMode::ProblemSolving).await?,
                        "exploration" => self.switch_mode(OperationalMode::Exploration).await?,
                        "consolidation" => self.switch_mode(OperationalMode::Consolidation).await?,
                        "planning" => self.switch_mode(OperationalMode::Planning).await?,
                        _ => self.switch_mode(OperationalMode::Custom(mode.to_string())).await?,
                    }
                }
                Ok(())
            },
            "system_alert" => {
                // Handle system alerts (e.g., low resources)
                if let Some(alert_type) = message.payload.get("type").and_then(|v| v.as_str()) {
                    match alert_type {
                        "low_resources" => {
                            debug!("System alert: Low resources");
                            // Switch to a mode that uses fewer resources
                            self.switch_mode(OperationalMode::Consolidation).await?;
                        },
                        "high_priority_task" => {
                            debug!("System alert: High priority task");
                            // Switch to problem solving mode
                            self.switch_mode(OperationalMode::ProblemSolving).await?;
                        },
                        _ => {
                            debug!("Unknown system alert type: {}", alert_type);
                        }
                    }
                }
                Ok(())
            },
            _ => {
                // Unknown message type
                warn!("Received unknown message type: {}", message.message_type);
                Ok(())
            }
        }
    }
    
    async fn collect_metrics(&self) -> Result<ComponentMetrics, ComponentError> {
        // Gather current metrics
        let mut metrics = self.base.get_metrics();
        
        // Get operational statistics
        let stats = self.stats.read().await;
        let mode = self.mode.read().await;
        
        // Add operational-specific metrics
        let custom_metrics = json!({
            "current_mode": self.get_mode_name(&mode),
            "task_queue_length": self.task_queue.lock().await.len(),
            "running_tasks": self.running_tasks.lock().await.len(),
            "scheduled_tasks": self.scheduled_tasks.lock().await.len(),
            "active_goals": self.goals.read().await.iter()
                .filter(|g| g.status == GoalStatus::Pending || g.status == GoalStatus::InProgress)
                .count(),
            "autonomous_enabled": self.autonomous_settings.enabled,
            "tasks_processed": stats.total_tasks_processed,
            "mode_switches": stats.mode_switches,
            "resource_usage": self.resource_allocation.allocated_resources,
        });
        
        metrics.custom_metrics = custom_metrics;
        
        Ok(metrics)
    }
    
    async fn export_state(&self) -> Result<serde_json::Value, ComponentError> {
        // Export component state for persistence
        let state = json!({
            "current_mode": format!("{:?}", *self.mode.read().await),
            "task_queue_length": self.task_queue.lock().await.len(),
            "running_tasks": self.running_tasks.lock().await.len(),
            "resource_allocation": self.resource_allocation,
            "autonomous_settings": {
                "enabled": self.autonomous_settings.enabled,
                "mode_switching": format!("{:?}", self.autonomous_settings.mode_switching),
            },
            "stats": *self.stats.read().await,
            "recovery_data": *self.recovery_data.read().await,
            "base": {
                "id": self.base.id,
                "component_type": self.base.component_type,
                "status": format!("{:?}", self.base.status),
                "metrics": {
                    "tasks_processed": self.base.metrics.tasks_processed,
                    "tasks_failed": self.base.metrics.tasks_failed,
                    "avg_processing_time": self.base.metrics.avg_processing_time,
                    "custom_metrics": self.base.metrics.custom_metrics,
                },
            }
        });
        
        Ok(state)
    }
    
    async fn import_state(&mut self, state: serde_json::Value) -> Result<(), ComponentError> {
        // Import previously exported state
        if let Some(mode_str) = state.get("current_mode").and_then(|v| v.as_str()) {
            match mode_str {
                "ProblemSolving" => *self.mode.write().await = OperationalMode::ProblemSolving,
                "Exploration" => *self.mode.write().await = OperationalMode::Exploration,
                "Consolidation" => *self.mode.write().await = OperationalMode::Consolidation,
                "Planning" => *self.mode.write().await = OperationalMode::Planning,
                _ => {
                    if mode_str.starts_with("Custom") {
                        let name = mode_str.trim_start_matches("Custom(").trim_end_matches(")").to_string();
                        *self.mode.write().await = OperationalMode::Custom(name);
                    }
                }
            }
        }
        
        if let Some(enabled) = state.get("autonomous_settings").and_then(|v| v.get("enabled")).and_then(|v| v.as_bool()) {
            self.autonomous_settings.enabled = enabled;
        }
        
        // Recovery data would be imported here in a full implementation
        
        Ok(())
    }
    
    fn get_info(&self) -> serde_json::Value {
        json!({
            "id": self.base.id,
            "type": self.base.component_type,
            "status": format!("{:?}", self.base.status),
            "current_mode": "ProblemSolving", // Would get the actual mode in a thread-safe way
            "autonomous_enabled": self.autonomous_settings.enabled,
            "tasks_processed": self.base.metrics.tasks_processed,
        })
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl OperationalStateInstance {
    // Background loops
    
    /// Task processing loop
    async fn task_processing_loop(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
        
        loop {
            interval.tick().await;
            
            // Skip if paused or shutting down
            if self.base.status != ComponentStatus::Running {
                continue;
            }
            
            // Process scheduled tasks
            if let Err(e) = self.check_scheduled_tasks().await {
                error!("Error checking scheduled tasks: {:?}", e);
            }
            
            // Get next task if any
            if let Some(task) = self.get_next_task().await {
                match self.start_task(&task).await {
                    Ok(_) => {
                        // Simulate task execution
                        let task_id = task.task.id.clone();
                        let instance = self.clone();
                        
                        tokio::spawn(async move {
                            // Simulate processing time
                            let duration = task.estimated_duration * 0.1; // Scale down for demonstration
                            tokio::time::sleep(std::time::Duration::from_millis(duration as u64)).await;
                            
                            // Update progress periodically
                            for progress in [0.25, 0.5, 0.75, 1.0] {
                                if let Err(e) = instance.update_task_progress(&task_id, progress).await {
                                    error!("Error updating task progress: {:?}", e);
                                }
                                
                                if progress < 1.0 {
                                    tokio::time::sleep(std::time::Duration::from_millis((duration / 4.0) as u64)).await;
                                }
                            }
                            
                            // Complete the task
                            if let Err(e) = instance.complete_task(
                                &task_id,
                                TaskStatus::Completed,
                                Some(json!({ "result": "success" }))
                            ).await {
                                error!("Error completing task: {:?}", e);
                            }
                        });
                    },
                    Err(e) => {
                        error!("Error starting task: {:?}", e);
                    }
                }
            }
        }
    }
    
    /// Autonomous operations loop
    async fn autonomous_operations_loop(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
        
        loop {
            interval.tick().await;
            
            // Skip if paused or shutting down
            if self.base.status != ComponentStatus::Running {
                continue;
            }
            
            // Skip if autonomous operations are disabled
            if !self.autonomous_settings.enabled {
                continue;
            }
            
            if let Err(e) = self.run_autonomous_operations().await {
                error!("Error running autonomous operations: {:?}", e);
            }
        }
    }
    
    /// Scheduled task checker loop
    async fn scheduled_task_checker_loop(&self) {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        
        loop {
            interval.tick().await;
            
            // Skip if paused or shutting down
            if self.base.status != ComponentStatus::Running {
                continue;
            }
            
            if let Err(e) = self.check_scheduled_tasks().await {
                error!("Error checking scheduled tasks: {:?}", e);
            }
        }
    }
}

impl Default for OperationalStateInstance {
    fn default() -> Self {
        Self::new()
    }
}

// Tests for OperationalStateInstance
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test creating a new operational state instance
    #[test]
    fn test_new_instance() {
        let instance = OperationalStateInstance::new();
        assert_eq!(instance.base.id, "operational_state");
        assert_eq!(instance.available_modes.len(), 4);
    }
    
    // Additional tests would be implemented here
}