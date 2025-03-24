use std::any::Any;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::core::component::{
    Component, ComponentStatus, ComponentMessage, ComponentTask, 
    ComponentError, ComponentMetrics, ComponentConfig, ResourceLimits
};

/// Base functionality for all component instances
pub struct BaseComponent {
    /// Component identifier
    pub id: String,
    /// Component type name
    pub component_type: String,
    /// Current status
    pub status: ComponentStatus,
    /// Configuration
    pub config: Option<ComponentConfig>,
    /// Resource usage
    pub resource_usage: ResourceUsage,
    /// Performance metrics
    pub metrics: PerformanceMetrics,
    /// Log of recent events
    pub event_log: Arc<Mutex<Vec<ComponentEvent>>>,
}

/// Resource usage tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in MB
    pub memory_usage: f64,
    /// Storage usage in MB
    pub storage_usage: f64,
    /// Network usage in KB/s
    pub network_usage: f64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Performance metrics tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Number of tasks processed
    pub tasks_processed: u64,
    /// Number of tasks failed
    pub tasks_failed: u64,
    /// Average processing time in ms
    pub avg_processing_time: f64,
    /// Last processing time in ms
    pub last_processing_time: f64,
    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Component event for logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: EventType,
    /// Event description
    pub description: String,
    /// Associated data
    pub data: Option<serde_json::Value>,
}

/// Types of component events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    /// Initialization
    Initialization,
    /// State change
    StateChange,
    /// Task processing
    TaskProcessing,
    /// Error occurred
    Error,
    /// Warning
    Warning,
    /// Informational event
    Info,
    /// Security-related event
    Security,
    /// Resource-related event
    Resource,
}

impl BaseComponent {
    /// Create a new base component
    pub fn new(id: &str, component_type: &str) -> Self {
        Self {
            id: id.to_string(),
            component_type: component_type.to_string(),
            status: ComponentStatus::Initialized,
            config: None,
            resource_usage: ResourceUsage {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                storage_usage: 0.0,
                network_usage: 0.0,
                last_updated: Utc::now(),
            },
            metrics: PerformanceMetrics {
                tasks_processed: 0,
                tasks_failed: 0,
                avg_processing_time: 0.0,
                last_processing_time: 0.0,
                custom_metrics: HashMap::new(),
                last_updated: Utc::now(),
            },
            event_log: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Log a component event
    pub async fn log_event(&self, event_type: EventType, description: &str, data: Option<serde_json::Value>) {
        let event = ComponentEvent {
            timestamp: Utc::now(),
            event_type,
            description: description.to_string(),
            data,
        };
        
        let mut log = self.event_log.lock().await;
        log.push(event.clone());
        
        // Trim event log if it gets too large
        if log.len() > 1000 {
            log.drain(0..500);
        }
        
        // Log to the system logger as well
        match event.event_type {
            EventType::Error => error!("{}: {}", self.id, description),
            EventType::Warning => warn!("{}: {}", self.id, description),
            _ => debug!("{}: {}", self.id, description),
        }
    }
    
    /// Update resource usage
    pub fn update_resource_usage(&mut self, cpu: f64, memory: f64, storage: f64, network: f64) {
        self.resource_usage = ResourceUsage {
            cpu_usage: cpu,
            memory_usage: memory,
            storage_usage: storage,
            network_usage: network,
            last_updated: Utc::now(),
        };
    }
    
    /// Record task processing time
    pub fn record_task_processing(&mut self, success: bool, processing_time_ms: f64) {
        if success {
            self.metrics.tasks_processed += 1;
        } else {
            self.metrics.tasks_failed += 1;
        }
        
        // Update average processing time using a weighted approach
        let total_tasks = self.metrics.tasks_processed + self.metrics.tasks_failed;
        if total_tasks > 1 {
            self.metrics.avg_processing_time = (
                self.metrics.avg_processing_time * (total_tasks - 1) as f64 + processing_time_ms
            ) / total_tasks as f64;
        } else {
            self.metrics.avg_processing_time = processing_time_ms;
        }
        
        self.metrics.last_processing_time = processing_time_ms;
        self.metrics.last_updated = Utc::now();
    }
    
    /// Set a custom metric value
    pub fn set_custom_metric(&mut self, name: &str, value: f64) {
        self.metrics.custom_metrics.insert(name.to_string(), value);
        self.metrics.last_updated = Utc::now();
    }
    
    /// Get recent events
    pub async fn get_recent_events(&self, limit: usize) -> Vec<ComponentEvent> {
        let log = self.event_log.lock().await;
        let start = if log.len() > limit {
            log.len() - limit
        } else {
            0
        };
        
        log[start..].to_vec()
    }
    
    /// Get component metrics
    pub fn get_metrics(&self) -> ComponentMetrics {
        ComponentMetrics {
            timestamp: Utc::now(),
            cpu_usage: self.resource_usage.cpu_usage,
            memory_usage: self.resource_usage.memory_usage,
            tasks_processed: self.metrics.tasks_processed,
            avg_processing_time: self.metrics.avg_processing_time,
            custom_metrics: serde_json::to_value(&self.metrics.custom_metrics).unwrap_or(serde_json::Value::Null),
        }
    }
}

/// Helper function to extract task parameters
pub fn extract_task_param<T: for<'de> Deserialize<'de>>(
    task: &ComponentTask,
    param_name: &str
) -> Result<T, ComponentError> {
    if let Some(value) = task.parameters.get(param_name) {
        match serde_json::from_value(value.clone()) {
            Ok(parsed) => Ok(parsed),
            Err(e) => Err(ComponentError::ValidationError(format!(
                "Failed to parse parameter '{}': {}", param_name, e
            ))),
        }
    } else {
        Err(ComponentError::ValidationError(format!(
            "Required parameter not found: '{}'", param_name
        )))
    }
}

/// Helper function to measure execution time
pub async fn measure_execution_time<F, T, E>(f: F) -> (Result<T, E>, f64)
where
    F: std::future::Future<Output = Result<T, E>>,
{
    let start = std::time::Instant::now();
    let result = f.await;
    let duration = start.elapsed();
    let duration_ms = duration.as_secs_f64() * 1000.0;
    
    (result, duration_ms)
}