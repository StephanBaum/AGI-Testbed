use std::any::Any;
use std::error::Error;
use std::fmt::{Debug, Display};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Represents the current status of a component
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentStatus {
    /// Component is initialized but not yet started
    Initialized,
    /// Component is running and operational
    Running,
    /// Component is paused but can be resumed
    Paused,
    /// Component is in the process of shutting down
    ShuttingDown,
    /// Component has encountered an error
    Error(String),
}

impl Display for ComponentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentStatus::Initialized => write!(f, "Initialized"),
            ComponentStatus::Running => write!(f, "Running"),
            ComponentStatus::Paused => write!(f, "Paused"),
            ComponentStatus::ShuttingDown => write!(f, "Shutting Down"),
            ComponentStatus::Error(err) => write!(f, "Error: {}", err),
        }
    }
}

/// Metric data collected from components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetrics {
    /// Timestamp when metrics were collected
    pub timestamp: DateTime<Utc>,
    /// CPU usage percentage (0-100)
    pub cpu_usage: f64,
    /// Memory usage in megabytes
    pub memory_usage: f64,
    /// Number of tasks processed since last metrics collection
    pub tasks_processed: u64,
    /// Average task processing time in milliseconds
    pub avg_processing_time: f64,
    /// Component-specific metrics as key-value pairs
    pub custom_metrics: serde_json::Value,
}

/// Error type for component operations
#[derive(Debug)]
pub enum ComponentError {
    /// Error during initialization
    InitializationError(String),
    /// Error during task processing
    ProcessingError(String),
    /// Error during state persistence
    PersistenceError(String),
    /// Error during component communication
    CommunicationError(String),
    /// Validation error
    ValidationError(String),
    /// Resource allocation error
    ResourceError(String),
    /// Security/permission error
    SecurityError(String),
    /// Component not in the expected state
    InvalidStateError(String),
}

impl Display for ComponentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComponentError::InitializationError(msg) => write!(f, "Initialization error: {}", msg),
            ComponentError::ProcessingError(msg) => write!(f, "Processing error: {}", msg),
            ComponentError::PersistenceError(msg) => write!(f, "Persistence error: {}", msg),
            ComponentError::CommunicationError(msg) => write!(f, "Communication error: {}", msg),
            ComponentError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ComponentError::ResourceError(msg) => write!(f, "Resource error: {}", msg),
            ComponentError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            ComponentError::InvalidStateError(msg) => write!(f, "Invalid state: {}", msg),
        }
    }
}

impl Error for ComponentError {}

/// Message type for inter-component communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMessage {
    /// Unique identifier for the message
    pub id: String,
    /// Component that sent the message
    pub sender: String,
    /// Target component (or broadcast if None)
    pub target: Option<String>,
    /// Message type identifier
    pub message_type: String,
    /// Message content as JSON
    pub payload: serde_json::Value,
    /// Timestamp when message was created
    pub timestamp: DateTime<Utc>,
    /// Priority level (higher is more urgent)
    pub priority: u8,
}

/// Task assignment for components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentTask {
    /// Unique identifier for the task
    pub id: String,
    /// Description of the task
    pub description: String,
    /// Task parameters as JSON
    pub parameters: serde_json::Value,
    /// Priority level (higher is more urgent)
    pub priority: u8,
    /// Task creation timestamp
    pub created_at: DateTime<Utc>,
    /// Deadline for task completion (if applicable)
    pub deadline: Option<DateTime<Utc>>,
}

/// Component configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentConfig {
    /// Component identifier
    pub id: String,
    /// Component display name
    pub name: String,
    /// Configuration parameters as JSON
    pub parameters: serde_json::Value,
    /// Resource allocation limits
    pub resource_limits: ResourceLimits,
}

/// Resource limits for components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_percent: f64,
    /// Maximum memory usage in megabytes
    pub max_memory_mb: f64,
    /// Maximum storage usage in megabytes
    pub max_storage_mb: Option<f64>,
    /// Maximum tasks per second
    pub max_tasks_per_second: Option<f64>,
}

/// Core trait for all AGI components
#[async_trait]
pub trait Component: Send + Sync + Debug {
    /// Returns the component identifier
    fn id(&self) -> &str;
    
    /// Returns the component type name
    fn component_type(&self) -> &str;
    
    /// Returns the current status of the component
    fn status(&self) -> ComponentStatus;
    
    /// Initialize the component with the given configuration
    async fn initialize(&mut self, config: ComponentConfig) -> Result<(), ComponentError>;
    
    /// Start the component
    async fn start(&mut self) -> Result<(), ComponentError>;
    
    /// Pause the component
    async fn pause(&mut self) -> Result<(), ComponentError>;
    
    /// Resume the component after being paused
    async fn resume(&mut self) -> Result<(), ComponentError>;
    
    /// Shut down the component
    async fn shutdown(&mut self) -> Result<(), ComponentError>;
    
    /// Process a task
    async fn process_task(&mut self, task: ComponentTask) -> Result<serde_json::Value, ComponentError>;
    
    /// Handle a message from another component
    async fn handle_message(&mut self, message: ComponentMessage) -> Result<(), ComponentError>;
    
    /// Collect metrics from the component
    async fn collect_metrics(&self) -> Result<ComponentMetrics, ComponentError>;
    
    /// Export the current state of the component (for persistence)
    async fn export_state(&self) -> Result<serde_json::Value, ComponentError>;
    
    /// Import a previously exported state
    async fn import_state(&mut self, state: serde_json::Value) -> Result<(), ComponentError>;
    
    /// Get component-specific information
    fn get_info(&self) -> serde_json::Value;
    
    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;
    
    /// Convert to mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}