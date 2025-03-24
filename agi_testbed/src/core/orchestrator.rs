use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::core::component::{
    Component, ComponentStatus, ComponentMessage, ComponentTask, ComponentError, ComponentConfig
};

/// The Core Orchestrator is the central coordination unit of the AGI Testbed.
/// It manages inter-instance communication, ensures operational mode persistence,
/// and enforces safety & alignment rules.
#[derive(Debug)]
pub struct Orchestrator {
    /// Map of component instances by their ID
    instances: HashMap<String, Arc<RwLock<dyn Component>>>,
    /// System-wide message channel
    message_tx: Option<broadcast::Sender<ComponentMessage>>,
    /// Task queue for scheduling
    task_queue: Vec<ComponentTask>,
    /// System-wide configuration
    config: OrchestratorConfig,
    /// Current system status
    status: SystemStatus,
    /// System metrics history
    metrics_history: Vec<SystemMetrics>,
    /// Security and alignment policies
    security_policies: SecurityPolicies,
}

/// Configuration for the orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Maximum number of concurrent tasks
    pub max_concurrent_tasks: usize,
    /// Task queue capacity
    pub task_queue_capacity: usize,
    /// Metrics collection interval in seconds
    pub metrics_interval_secs: u64,
    /// Message buffer capacity
    pub message_buffer_capacity: usize,
    /// Enable strict security mode
    pub strict_security: bool,
    /// Data persistence settings
    pub persistence: PersistenceConfig,
}

/// System-wide persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Enable state persistence
    pub enabled: bool,
    /// Persistence interval in seconds
    pub interval_secs: u64,
    /// Storage location
    pub storage_path: String,
}

/// Overall system status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    /// Current orchestrator state
    pub state: OrchestratorState,
    /// Last state update time
    pub last_updated: DateTime<Utc>,
    /// Active component count
    pub active_components: usize,
    /// Error messages if any
    pub errors: Vec<String>,
    /// Warning messages
    pub warnings: Vec<String>,
}

/// Orchestrator operational states
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrchestratorState {
    /// System is initializing
    Initializing,
    /// System is running normally
    Running,
    /// System is in maintenance mode
    Maintenance,
    /// System is shutting down
    ShuttingDown,
    /// System is in error state
    Error,
    /// System is in sandbox/safe mode
    Sandbox,
}

/// System-wide metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Timestamp of metrics collection
    pub timestamp: DateTime<Utc>,
    /// System-wide CPU usage percentage
    pub total_cpu_usage: f64,
    /// System-wide memory usage in MB
    pub total_memory_usage: f64,
    /// Tasks processed per second
    pub tasks_per_second: f64,
    /// Message throughput per second
    pub messages_per_second: f64,
    /// Current task queue size
    pub queue_size: usize,
    /// Component-specific metrics
    pub component_metrics: HashMap<String, serde_json::Value>,
}

/// Security and alignment policies
#[derive(Debug, Clone)]
pub struct SecurityPolicies {
    /// Component access control matrix
    access_control: HashMap<String, Vec<String>>,
    /// Resource limits by component
    resource_limits: HashMap<String, f64>,
    /// Blocked operations
    blocked_operations: Vec<String>,
    /// Requires human approval
    human_approval_required: Vec<String>,
}

impl Orchestrator {
    /// Create a new orchestrator with default settings
    pub fn new() -> Self {
        // Create a broadcast channel for system-wide messaging
        let (tx, _) = broadcast::channel(1000);
        
        Self {
            instances: HashMap::new(),
            message_tx: Some(tx),
            task_queue: Vec::new(),
            config: OrchestratorConfig {
                max_concurrent_tasks: 100,
                task_queue_capacity: 1000,
                metrics_interval_secs: 10,
                message_buffer_capacity: 1000,
                strict_security: true,
                persistence: PersistenceConfig {
                    enabled: true,
                    interval_secs: 300,
                    storage_path: "./data/state".to_string(),
                },
            },
            status: SystemStatus {
                state: OrchestratorState::Initializing,
                last_updated: Utc::now(),
                active_components: 0,
                errors: Vec::new(),
                warnings: Vec::new(),
            },
            metrics_history: Vec::new(),
            security_policies: SecurityPolicies {
                access_control: HashMap::new(),
                resource_limits: HashMap::new(),
                blocked_operations: Vec::new(),
                human_approval_required: Vec::new(),
            },
        }
    }
    
    /// Register a component instance with the orchestrator
    pub fn register_instance(&mut self, id: &str, instance: Arc<RwLock<dyn Component>>) {
        if self.instances.contains_key(id) {
            warn!("Replacing existing instance with ID: {}", id);
        }
        self.instances.insert(id.to_string(), instance);
        self.status.active_components = self.instances.len();
        info!("Registered component: {}", id);
    }
    
    /// Remove a component instance from the orchestrator
    pub fn unregister_instance(&mut self, id: &str) -> bool {
        let result = self.instances.remove(id).is_some();
        if result {
            self.status.active_components = self.instances.len();
            info!("Unregistered component: {}", id);
        } else {
            warn!("Attempted to unregister non-existent component: {}", id);
        }
        result
    }
    
    /// Get a reference to a component instance by ID
    pub fn get_instance(&self, id: &str) -> Option<&Arc<RwLock<dyn Component>>> {
        self.instances.get(id)
    }
    
    /// Get all registered component instances
    pub fn get_all_instances(&self) -> &HashMap<String, Arc<RwLock<dyn Component>>> {
        &self.instances
    }
    
    /// Start all component instances
    pub async fn start_all(&mut self) -> Result<(), ComponentError> {
        info!("Starting all component instances...");
        self.status.state = OrchestratorState::Running;
        self.status.last_updated = Utc::now();
        
        // Create a message receiver for each component
        if let Some(tx) = &self.message_tx {
            // Setup message distribution to components
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                Self::distribute_messages(tx_clone).await;
            });
        }
        
        // Start metrics collection
        let metrics_interval = self.config.metrics_interval_secs;
        let instances = self.instances.clone();
        tokio::spawn(async move {
            Self::collect_metrics(instances, metrics_interval).await;
        });
        
        // Start all component instances
        for (id, instance) in &self.instances {
            match instance.write().await.start().await {
                Ok(_) => info!("Started component: {}", id),
                Err(e) => {
                    error!("Failed to start component {}: {:?}", id, e);
                    self.status.errors.push(format!("Failed to start {}: {}", id, e));
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Stop all component instances
    pub async fn stop_all(&mut self) -> Result<(), ComponentError> {
        info!("Stopping all component instances...");
        self.status.state = OrchestratorState::ShuttingDown;
        self.status.last_updated = Utc::now();
        
        // Stop all component instances
        for (id, instance) in &self.instances {
            match instance.write().await.shutdown().await {
                Ok(_) => info!("Stopped component: {}", id),
                Err(e) => {
                    error!("Failed to stop component {}: {:?}", id, e);
                    self.status.errors.push(format!("Failed to stop {}: {}", id, e));
                    return Err(e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Broadcast a message to all components
    pub async fn broadcast_message(&self, message: ComponentMessage) -> Result<(), ComponentError> {
        if let Some(tx) = &self.message_tx {
            match tx.send(message.clone()) {
                Ok(_) => {
                    debug!("Broadcast message: {:?}", message);
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to broadcast message: {:?}", e);
                    Err(ComponentError::CommunicationError(format!(
                        "Failed to broadcast message: {}", e
                    )))
                }
            }
        } else {
            Err(ComponentError::CommunicationError(
                "Message broadcaster not initialized".to_string()
            ))
        }
    }
    
    /// Send a message to a specific component
    pub async fn send_message(&self, target: &str, message: ComponentMessage) -> Result<(), ComponentError> {
        if let Some(instance) = self.instances.get(target) {
            match instance.write().await.handle_message(message).await {
                Ok(_) => {
                    debug!("Sent message to {}", target);
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to send message to {}: {:?}", target, e);
                    Err(e)
                }
            }
        } else {
            Err(ComponentError::CommunicationError(format!(
                "Component not found: {}", target
            )))
        }
    }
    
    /// Submit a task to be processed by a specific component
    pub async fn submit_task(&mut self, component_id: &str, task: ComponentTask) -> Result<serde_json::Value, ComponentError> {
        if let Some(instance) = self.instances.get(component_id) {
            let result = instance.write().await.process_task(task).await?;
            Ok(result)
        } else {
            Err(ComponentError::ProcessingError(format!(
                "Component not found: {}", component_id
            )))
        }
    }
    
    /// Get the current system status
    pub fn get_status(&self) -> &SystemStatus {
        &self.status
    }
    
    /// Get the latest system metrics
    pub fn get_latest_metrics(&self) -> Option<&SystemMetrics> {
        self.metrics_history.last()
    }
    
    /// Get historical metrics within a time range
    pub fn get_historical_metrics(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>
    ) -> Vec<&SystemMetrics> {
        self.metrics_history
            .iter()
            .filter(|m| m.timestamp >= start_time && m.timestamp <= end_time)
            .collect()
    }
    
    /// Helper function to distribute messages to components
    async fn distribute_messages(tx: broadcast::Sender<ComponentMessage>) {
        // This would be implemented to handle message routing logic
        // For the MVP, we'll use the simpler direct message approach
    }
    
    /// Helper function to periodically collect metrics from all components
    async fn collect_metrics(
        instances: HashMap<String, Arc<RwLock<dyn Component>>>,
        interval_secs: u64
    ) {
        // This would be implemented to periodically collect and store metrics
        // For the MVP, this is a placeholder
    }
    
    /// Create a unique task ID
    pub fn create_task_id(&self) -> String {
        Uuid::new_v4().to_string()
    }
    
    /// Check if an operation is allowed by security policies
    pub fn is_operation_allowed(&self, component_id: &str, operation: &str) -> bool {
        // For the MVP, we'll implement a simple check
        !self.security_policies.blocked_operations.contains(&operation.to_string())
    }
    
    /// Get component configurations
    pub fn get_component_configs(&self) -> HashMap<String, ComponentConfig> {
        let mut configs = HashMap::new();
        // In a full implementation, this would return the actual configurations
        // For the MVP, we'll return empty configs
        configs
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

// Tests for the Orchestrator
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test the creation of a new orchestrator
    #[test]
    fn test_new_orchestrator() {
        let orchestrator = Orchestrator::new();
        assert_eq!(orchestrator.status.state, OrchestratorState::Initializing);
        assert_eq!(orchestrator.instances.len(), 0);
    }
    
    // Additional tests would be implemented here
}