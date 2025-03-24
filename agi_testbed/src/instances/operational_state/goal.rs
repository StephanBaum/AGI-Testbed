//! Goal management functionality for the Operational State Instance
//!
//! This module implements goal creation, tracking, and progress management
//! operations for the Operational State instance.

use std::collections::HashMap;
use log::{info, debug};
use serde_json::json;
use chrono::Utc;

use crate::core::component::ComponentError;
use crate::instances::common::EventType;

use super::models::*;
use super::instance::OperationalStateInstance;

impl OperationalStateInstance {
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
                    *pending_count = pending_count.saturating_sub(1);
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
    
    /// Generate goals based on current system state
    pub async fn generate_goals(&self) -> Result<(), ComponentError> {
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
    
    /// Set a deadline for a goal
    pub async fn set_goal_deadline(&self, goal_id: &str, deadline: chrono::DateTime<Utc>) -> Result<(), ComponentError> {
        let mut goals = self.goals.write().await;
        
        if let Some(goal) = goals.iter_mut().find(|g| g.id == goal_id) {
            goal.deadline = Some(deadline);
            
            // Log the deadline setting
            self.base.log_event(
                EventType::Info, 
                &format!("Set deadline for goal: {}", goal.description), 
                Some(json!({
                    "id": goal_id,
                    "deadline": deadline,
                }))
            ).await;
            
            Ok(())
        } else {
            Err(ComponentError::ValidationError(format!(
                "Goal not found: {}", goal_id
            )))
        }
    }
    
    /// Link a task to a goal
    pub async fn link_task_to_goal(&self, goal_id: &str, task_id: &str) -> Result<(), ComponentError> {
        let mut goals = self.goals.write().await;
        
        if let Some(goal) = goals.iter_mut().find(|g| g.id == goal_id) {
            // Check if task is already linked
            if !goal.related_tasks.contains(&task_id.to_string()) {
                goal.related_tasks.push(task_id.to_string());
                
                debug!("Linked task {} to goal {}", task_id, goal_id);
            }
            
            Ok(())
        } else {
            Err(ComponentError::ValidationError(format!(
                "Goal not found: {}", goal_id
            )))
        }
    }
    
    /// Create a child goal (sub-goal)
    pub async fn create_child_goal(&self, parent_id: &str, description: &str, priority: u8) -> Result<String, ComponentError> {
        // First check if parent exists
        {
            let goals = self.goals.read().await;
            if !goals.iter().any(|g| g.id == parent_id) {
                return Err(ComponentError::ValidationError(format!(
                    "Parent goal not found: {}", parent_id
                )));
            }
        }
        
        // Create new goal
        let child_id = self.create_goal(description, priority).await?;
        
        // Update parent-child relationships
        {
            let mut goals = self.goals.write().await;
            
            // Set parent on child
            if let Some(child) = goals.iter_mut().find(|g| g.id == child_id) {
                child.parent_goal = Some(parent_id.to_string());
            }
            
            // Add child to parent
            if let Some(parent) = goals.iter_mut().find(|g| g.id == parent_id) {
                parent.child_goals.push(child_id.clone());
            }
        }
        
        debug!("Created child goal {} under parent {}", child_id, parent_id);
        
        Ok(child_id)
    }
    
    /// Change goal status
    pub async fn change_goal_status(&self, goal_id: &str, status: GoalStatus) -> Result<(), ComponentError> {
        let mut goals = self.goals.write().await;
        
        if let Some(goal) = goals.iter_mut().find(|g| g.id == goal_id) {
            let old_status = goal.status.clone();
            
            // Skip if status is the same
            if old_status == status {
                return Ok(());
            }
            
            goal.status = status.clone();
            
            // Update stats
            let mut stats = self.stats.write().await;
            let old_count = stats.goal_stats.goals_by_status.entry(format!("{:?}", old_status)).or_insert(0);
            *old_count = old_count.saturating_sub(1);
            let new_count = stats.goal_stats.goals_by_status.entry(format!("{:?}", status)).or_insert(0);
            *new_count += 1;
            
            // If completed, update completion stats
            if status == GoalStatus::Completed {
                let duration_secs = (Utc::now() - goal.created_at).num_seconds() as f64;
                let completed_count = *new_count as f64;
                stats.goal_stats.avg_completion_time_secs = (
                    stats.goal_stats.avg_completion_time_secs * (completed_count - 1.0) + duration_secs
                ) / completed_count;
                
                // Update success rate
                let total = stats.goal_stats.total_goals as f64;
                if total > 0.0 {
                    stats.goal_stats.success_rate = completed_count / total;
                }
            }
            
            // Log the status change
            self.base.log_event(
                EventType::StateChange, 
                &format!("Changed goal status: {}", goal.description), 
                Some(json!({
                    "id": goal_id,
                    "old_status": format!("{:?}", old_status),
                    "new_status": format!("{:?}", status),
                }))
            ).await;
            
            Ok(())
        } else {
            Err(ComponentError::ValidationError(format!(
                "Goal not found: {}", goal_id
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Goal-related tests would go here
}