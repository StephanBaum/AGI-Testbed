use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// System status response
#[derive(Serialize)]
pub struct SystemStatusResponse {
    pub status: String,
    pub orchestrator_state: String,
    pub active_components: usize,
    pub uptime_seconds: u64,
    pub version: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// System metrics response
#[derive(Serialize)]
pub struct SystemMetricsResponse {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub component_metrics: HashMap<String, ComponentMetricsResponse>,
    pub timestamp: DateTime<Utc>,
}

/// Component metrics response
#[derive(Serialize)]
pub struct ComponentMetricsResponse {
    pub id: String,
    pub status: String,
    pub tasks_processed: u64,
    pub avg_processing_time: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub custom_metrics: serde_json::Value,
}

/// Component info response
#[derive(Serialize)]
pub struct ComponentInfoResponse {
    pub id: String,
    pub component_type: String,
    pub status: String,
    pub info: serde_json::Value,
}

/// Message request
#[derive(Deserialize)]
pub struct MessageRequest {
    pub target: Option<String>,
    pub message_type: String,
    pub payload: serde_json::Value,
    pub priority: Option<u8>,
}

/// Task request
#[derive(Deserialize)]
pub struct TaskRequest {
    pub description: String,
    pub operation: String,
    pub parameters: serde_json::Value,
    pub priority: Option<u8>,
}

/// Time scaling mode request
#[derive(Deserialize)]
pub struct TimeScalingModeRequest {
    pub mode: String,
    pub factor: Option<f64>,
}

/// Memory item request
#[derive(Deserialize)]
pub struct MemoryItemRequest {
    pub content: serde_json::Value,
    pub metadata: MemoryItemMetadata,
    pub target: Option<String>,
}

/// Memory item metadata
#[derive(Serialize, Deserialize)]
pub struct MemoryItemMetadata {
    pub importance: f64,
    pub processing_depth: f64,
    pub tags: Vec<String>,
    pub source: String,
    pub confidence: f64,
    pub related_ids: Vec<String>,
    pub custom: Option<serde_json::Value>,
}

/// Memory query request
#[derive(Deserialize)]
pub struct MemoryQueryRequest {
    pub id: Option<String>,
    pub tags: Option<Vec<String>>,
    pub content_query: Option<String>,
    pub min_importance: Option<f64>,
    pub query_both: Option<bool>,
    pub limit: Option<usize>,
}

/// Memory allocation request
#[derive(Deserialize)]
pub struct MemoryAllocationRequest {
    pub stm_allocation: f64,
}

/// Knowledge node request
#[derive(Deserialize)]
pub struct KnowledgeNodeRequest {
    pub label: String,
    pub properties: serde_json::Value,
    pub confidence: Option<f64>,
    pub source: Option<String>,
}

/// Knowledge edge request
#[derive(Deserialize)]
pub struct KnowledgeEdgeRequest {
    pub source_id: String,
    pub target_id: String,
    pub relationship: String,
    pub properties: serde_json::Value,
    pub confidence: Option<f64>,
    pub source: Option<String>,
    pub is_inferred: Option<bool>,
}

/// Knowledge query request
#[derive(Deserialize)]
pub struct KnowledgeQueryRequest {
    pub start_nodes: Vec<String>,
    pub pattern: Option<serde_json::Value>,
    pub text_query: Option<String>,
    pub min_confidence: Option<f64>,
    pub max_depth: Option<u32>,
    pub limit: Option<usize>,
}

/// Operational mode request
#[derive(Deserialize)]
pub struct OperationalModeRequest {
    pub mode: String,
}

/// Goal request
#[derive(Deserialize)]
pub struct GoalRequest {
    pub description: String,
    pub priority: Option<u8>,
    pub deadline: Option<DateTime<Utc>>,
}

/// Testbed run request
#[derive(Deserialize)]
pub struct TestbedRunRequest {
    pub testbed_id: String,
    pub parameters: serde_json::Value,
    pub run_name: Option<String>,
}

/// Generic response
#[derive(Serialize)]
pub struct GenericResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Error response
#[derive(Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub error_code: String,
}