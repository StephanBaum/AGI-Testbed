//! Task management functionality for the Operational State Instance
//!
//! This module implements task scheduling, execution, and management
//! operations for the Operational State instance.

use std::collections::HashMap;
use log::{info, warn, error, debug};
use serde_json::json;
use chrono::{DateTime, Utc, Duration};

use crate::core::component::ComponentError;
use crate::instances::common::EventType;

use super::models::*;
use super::instance::OperationalStateInstance;

impl OperationalStateInstance {
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

    /// Check if scheduled tasks need to be executed
    pub async fn check_scheduled_tasks(&self) -> Result<(), ComponentError> {
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
    pub fn calculate_next_occurrence(&self, last_time: DateTime<Utc>, recurrence: &RecurrencePattern) -> DateTime<Utc> {
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

    /// Update task priorities based on mode
    pub async fn update_task_priorities(&self, priorities: &HashMap<String, u8>) -> Result<(), ComponentError> {
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
    pub async fn resort_task_queue(&self) -> Result<(), ComponentError> {
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

    /// Capture the state of the current mode for later restoration
    pub async fn capture_mode_state(&self, mode: &OperationalMode) -> Result<(), ComponentError> {
        // In a real implementation, this would save state specific to the mode
        // For the MVP, we'll just log the mode change
        debug!("Capturing state for {} mode", self.get_mode_name(mode));
        Ok(())
    }

    /// Allocate resources for a task
    pub async fn allocate_resources(&self, task: &PrioritizedTask) -> Result<HashMap<String, f64>, ComponentError> {
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
    pub async fn release_resources(&self, resources: &HashMap<String, f64>) -> Result<(), ComponentError> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Task-related tests would go here
}