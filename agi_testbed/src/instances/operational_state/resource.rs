//! Resource management functionality for the Operational State Instance
//!
//! This module implements resource allocation, tracking, and optimization
//! for the Operational State instance.

use std::collections::HashMap;
use log::{debug, warn};
use serde_json::json;

use crate::core::component::ComponentError;
use crate::instances::common::EventType;

use super::models::*;
use super::instance::OperationalStateInstance;

impl OperationalStateInstance {
    /// Get current resource allocation
    pub async fn get_resource_allocation(&self) -> ResourceAllocation {
        self.resource_allocation.clone()
    }
    
    /// Update total resources available
    pub async fn update_total_resources(&self, resources: HashMap<String, f64>) -> Result<(), ComponentError> {
        let mut resource_allocation = self.resource_allocation.clone();
        
        // Update total resources
        for (resource, amount) in resources {
            resource_allocation.total_resources.insert(resource.clone(), amount);
        }
        
        // Update the resource allocation
        self.resource_allocation = resource_allocation;
        
        // Log the update
        self.base.log_event(
            EventType::ConfigChange, 
            "Updated total resources", 
            Some(json!(resources))
        ).await;
        
        Ok(())
    }
    
    /// Update resource allocation policy
    pub async fn update_allocation_policy(&self, resource: &str, policy: AllocationPolicy) -> Result<(), ComponentError> {
        // Check if resource exists
        if !self.resource_allocation.total_resources.contains_key(resource) {
            return Err(ComponentError::ValidationError(format!(
                "Unknown resource: {}", resource
            )));
        }
        
        let mut resource_allocation = self.resource_allocation.clone();
        
        // Update policy
        resource_allocation.allocation_policies.insert(resource.to_string(), policy.clone());
        
        // Update the resource allocation
        self.resource_allocation = resource_allocation;
        
        // Log the update
        self.base.log_event(
            EventType::ConfigChange, 
            &format!("Updated allocation policy for {}", resource), 
            Some(json!({
                "resource": resource,
                "policy": format!("{:?}", policy),
            }))
        ).await;
        
        Ok(())
    }
    
    /// Update system resource reservation
    pub async fn update_system_reservation(&self, resources: HashMap<String, f64>) -> Result<(), ComponentError> {
        let mut resource_allocation = self.resource_allocation.clone();
        
        // Update system reservation
        for (resource, amount) in resources {
            // Check if resource exists
            if !resource_allocation.total_resources.contains_key(&resource) {
                return Err(ComponentError::ValidationError(format!(
                    "Unknown resource: {}", resource
                )));
            }
            
            // Check if reservation is valid (not more than total)
            let total = *resource_allocation.total_resources.get(&resource).unwrap_or(&0.0);
            if amount > total {
                return Err(ComponentError::ValidationError(format!(
                    "Reservation for {} exceeds total available ({})", resource, total
                )));
            }
            
            resource_allocation.system_reservation.insert(resource.clone(), amount);
        }
        
        // Update the resource allocation
        self.resource_allocation = resource_allocation;
        
        // Log the update
        self.base.log_event(
            EventType::ConfigChange, 
            "Updated system resource reservation", 
            Some(json!(resources))
        ).await;
        
        Ok(())
    }
    
    /// Get available resources (total - allocated - reserved)
    pub async fn get_available_resources(&self) -> HashMap<String, f64> {
        let mut available = HashMap::new();
        
        for (resource, &total) in &self.resource_allocation.total_resources {
            let allocated = *self.resource_allocation.allocated_resources.get(resource).unwrap_or(&0.0);
            let reserved = *self.resource_allocation.system_reservation.get(resource).unwrap_or(&0.0);
            
            let available_amount = (total - allocated - reserved).max(0.0);
            available.insert(resource.clone(), available_amount);
        }
        
        available
    }
    
    /// Check if there are enough resources for a task
    pub async fn has_resources_for_task(&self, task: &PrioritizedTask) -> (bool, HashMap<String, f64>) {
        let available = self.get_available_resources().await;
        let mut shortages = HashMap::new();
        let mut has_all = true;
        
        for (resource, &required) in &task.required_resources {
            if let Some(&available_amount) = available.get(resource) {
                if available_amount < required {
                    shortages.insert(resource.clone(), required - available_amount);
                    has_all = false;
                }
            } else {
                // Resource not found
                shortages.insert(resource.clone(), required);
                has_all = false;
            }
        }
        
        (has_all, shortages)
    }
    
    /// Optimize resource allocation based on queued tasks
    pub async fn optimize_resource_allocation(&self) -> Result<(), ComponentError> {
        debug!("Optimizing resource allocation");
        
        // Get current queue
        let queue = self.task_queue.lock().await;
        
        // Reset allocated resources (except for running tasks)
        let mut resource_allocation = self.resource_allocation.clone();
        let running_tasks = self.running_tasks.lock().await;
        
        // Start with resources used by running tasks
        for (_, task) in running_tasks.iter() {
            for (resource, &amount) in &task.allocated_resources {
                let current = resource_allocation.allocated_resources.entry(resource.clone()).or_insert(0.0);
                *current += amount;
            }
        }
        
        // Get total available resources
        let mut available = HashMap::new();
        for (resource, &total) in &resource_allocation.total_resources {
            let running = *resource_allocation.allocated_resources.get(resource).unwrap_or(&0.0);
            let reserved = *resource_allocation.system_reservation.get(resource).unwrap_or(&0.0);
            
            available.insert(resource.clone(), total - running - reserved);
        }
        
        // For each resource, allocate based on policy
        for (resource, &policy) in &resource_allocation.allocation_policies {
            let available_amount = *available.get(resource).unwrap_or(&0.0);
            if available_amount <= 0.0 {
                continue; // No resource available
            }
            
            // Get tasks that need this resource
            let tasks_needing_resource: Vec<_> = queue.iter()
                .filter(|task| task.required_resources.contains_key(resource))
                .collect();
            
            if tasks_needing_resource.is_empty() {
                continue; // No tasks need this resource
            }
            
            match policy {
                AllocationPolicy::EqualShare => {
                    // Divide equally among all tasks
                    let share = available_amount / tasks_needing_resource.len() as f64;
                    
                    for task in &tasks_needing_resource {
                        let required = *task.required_resources.get(resource).unwrap();
                        let allocated = required.min(share);
                        
                        debug!("Allocated {} of {} to task {} (Equal Share)", 
                               allocated, resource, task.task.id);
                    }
                },
                AllocationPolicy::PriorityBased => {
                    // Sort by priority (higher first)
                    let mut sorted_tasks = tasks_needing_resource.clone();
                    sorted_tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
                    
                    // Allocate in priority order
                    let mut remaining = available_amount;
                    
                    for task in sorted_tasks {
                        let required = *task.required_resources.get(resource).unwrap();
                        let allocated = required.min(remaining);
                        remaining -= allocated;
                        
                        debug!("Allocated {} of {} to task {} (Priority Based)", 
                               allocated, resource, task.task.id);
                        
                        if remaining <= 0.0 {
                            break;
                        }
                    }
                },
                AllocationPolicy::FCFS => {
                    // First come, first served - allocate in queue order
                    let mut remaining = available_amount;
                    
                    for task in &tasks_needing_resource {
                        let required = *task.required_resources.get(resource).unwrap();
                        let allocated = required.min(remaining);
                        remaining -= allocated;
                        
                        debug!("Allocated {} of {} to task {} (FCFS)", 
                               allocated, resource, task.task.id);
                        
                        if remaining <= 0.0 {
                            break;
                        }
                    }
                },
                AllocationPolicy::Custom(_) => {
                    // Custom policies would be implemented here
                    warn!("Custom allocation policy not fully implemented");
                }
            }
        }
        
        Ok(())
    }
    
    /// Add a new resource type
    pub async fn add_resource_type(&self, name: &str, total_amount: f64, policy: AllocationPolicy) -> Result<(), ComponentError> {
        let mut resource_allocation = self.resource_allocation.clone();
        
        // Check if resource already exists
        if resource_allocation.total_resources.contains_key(name) {
            return Err(ComponentError::ValidationError(format!(
                "Resource already exists: {}", name
            )));
        }
        
        // Add the new resource
        resource_allocation.total_resources.insert(name.to_string(), total_amount);
        resource_allocation.allocated_resources.insert(name.to_string(), 0.0);
        resource_allocation.allocation_policies.insert(name.to_string(), policy.clone());
        resource_allocation.system_reservation.insert(name.to_string(), total_amount * 0.1); // 10% default reservation
        
        // Update the resource allocation
        self.resource_allocation = resource_allocation;
        
        // Log the addition
        self.base.log_event(
            EventType::ConfigChange, 
            &format!("Added resource type: {}", name), 
            Some(json!({
                "name": name,
                "total_amount": total_amount,
                "policy": format!("{:?}", policy),
            }))
        ).await;
        
        Ok(())
    }
    
    /// Get resource usage statistics
    pub async fn get_resource_stats(&self) -> HashMap<String, ResourceStats> {
        let mut stats = HashMap::new();
        
        for (resource, &total) in &self.resource_allocation.total_resources {
            let allocated = *self.resource_allocation.allocated_resources.get(resource).unwrap_or(&0.0);
            let reserved = *self.resource_allocation.system_reservation.get(resource).unwrap_or(&0.0);
            
            let usage_percent = if total > 0.0 { (allocated / total) * 100.0 } else { 0.0 };
            let reserved_percent = if total > 0.0 { (reserved / total) * 100.0 } else { 0.0 };
            let available = (total - allocated - reserved).max(0.0);
            let available_percent = if total > 0.0 { (available / total) * 100.0 } else { 0.0 };
            
            stats.insert(resource.clone(), ResourceStats {
                total,
                allocated,
                reserved,
                available,
                usage_percent,
                reserved_percent,
                available_percent,
                policy: format!("{:?}", self.resource_allocation.allocation_policies.get(resource)
                    .unwrap_or(&AllocationPolicy::EqualShare)),
            });
        }
        
        stats
    }
}

/// Resource usage statistics
#[derive(Debug, Clone)]
pub struct ResourceStats {
    /// Total amount of the resource
    pub total: f64,
    /// Currently allocated amount
    pub allocated: f64,
    /// Reserved for system tasks
    pub reserved: f64,
    /// Available amount (total - allocated - reserved)
    pub available: f64,
    /// Usage as a percentage of total
    pub usage_percent: f64,
    /// Reserved as a percentage of total
    pub reserved_percent: f64,
    /// Available as a percentage of total
    pub available_percent: f64,
    /// Current allocation policy
    pub policy: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Resource-related tests would go here
}