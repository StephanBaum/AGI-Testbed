//! Core implementation of the Operational State Instance
//!
//! This file contains the main OperationalStateInstance struct and
//! the Component trait implementation.

use std::any::Any;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use async_trait::async_trait;
use log::{info, warn, error, debug};
use serde_json::json;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use rand;

use crate::core::component::{
    Component, ComponentStatus, ComponentMessage, ComponentTask, 
    ComponentError, ComponentMetrics, ComponentConfig
};
use crate::instances::common::{
    BaseComponent, EventType, measure_execution_time, extract_task_param
};

use super::models::*;

/// Persistent Operational State Instance
///
/// This component demonstrates autonomous operation in multiple modes:
/// problem-solving, exploration, consolidation, and planning.
/// It balances resources among concurrent tasks and maintains
/// continuous operation with various priorities.
#[derive(Debug)]
pub struct OperationalStateInstance {
    /// Base component functionality
    pub(crate) base: BaseComponent,
    /// Current operational mode
    pub(crate) mode: Arc<RwLock<OperationalMode>>,
    /// Available operational modes
    pub(crate) available_modes: Vec<OperationalModeConfig>,
    /// Task queue for scheduling
    pub(crate) task_queue: Arc<Mutex<VecDeque<PrioritizedTask>>>,
    /// Currently running tasks
    pub(crate) running_tasks: Arc<Mutex<HashMap<String, RunningTaskInfo>>>,
    /// Scheduled tasks for future execution
    pub(crate) scheduled_tasks: Arc<Mutex<Vec<ScheduledTask>>>,
    /// Resource allocation settings
    pub(crate) resource_allocation: ResourceAllocation,
    /// Task history
    pub(crate) task_history: Arc<Mutex<VecDeque<TaskHistoryEntry>>>,
    /// Current goals
    pub(crate) goals: Arc<RwLock<Vec<Goal>>>,
    /// Autonomous operation settings
    pub(crate) autonomous_settings: AutonomousSettings,
    /// Stats tracking
    pub(crate) stats: Arc<RwLock<OperationalStats>>,
    /// Interruption recovery data
    pub(crate) recovery_data: Arc<RwLock<RecoveryData>>,
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

    /// Get the name of an operational mode
    pub fn get_mode_name(&self, mode: &OperationalMode) -> String {
        match mode {
            OperationalMode::ProblemSolving => "Problem Solving".to_string(),
            OperationalMode::Exploration => "Exploration".to_string(),
            OperationalMode::Consolidation => "Consolidation".to_string(),
            OperationalMode::Planning => "Planning".to_string(),
            OperationalMode::Custom(name) => format!("Custom ({})", name),
        }
    }
    
    /// Get the configuration for a mode
    pub fn get_mode_config(&self, mode: &OperationalMode) -> Option<OperationalModeConfig> {
        self.available_modes.iter()
            .find(|m| m.mode == *mode)
            .cloned()
    }

    // Background loops
    
    /// Task processing loop
    pub async fn task_processing_loop(&self) {
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
    pub async fn autonomous_operations_loop(&self) {
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
    pub async fn scheduled_task_checker_loop(&self) {
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
                // For other operation types, delegate to specialized modules
                // This will be filled in after refactoring all the module functionality
                
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