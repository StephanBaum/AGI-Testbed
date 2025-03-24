use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use log::error;
use std::time::{SystemTime, UNIX_EPOCH};
use std::collections::HashMap;

use crate::web::server::AppState;
use crate::web::models::{SystemStatusResponse, SystemMetricsResponse, ComponentMetricsResponse, ComponentInfoResponse};
use crate::core::component::ComponentStatus;

/// Get the overall system status
pub async fn get_system_status(data: web::Data<AppState>) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    let status = orchestrator.get_status();
    
    let response = SystemStatusResponse {
        status: format!("{:?}", status.state),
        orchestrator_state: format!("{:?}", status.state),
        active_components: status.active_components,
        uptime_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        errors: status.errors.clone(),
        warnings: status.warnings.clone(),
    };
    
    HttpResponse::Ok().json(response)
}

/// Get the system metrics
pub async fn get_system_metrics(data: web::Data<AppState>) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    
    // Get the latest metrics from orchestrator
    let system_metrics = match orchestrator.get_latest_metrics() {
        Some(metrics) => metrics,
        None => {
            return HttpResponse::NotFound().json(web::Json(serde_json::json!({
                "success": false,
                "error": "No metrics available"
            })));
        }
    };
    
    // Create component metrics
    let mut component_metrics = HashMap::new();
    
    // Add time scaling component metrics
    if let Ok(metrics) = data.time_scaling.read().await.collect_metrics().await {
        component_metrics.insert("time_scaling".to_string(), ComponentMetricsResponse {
            id: "time_scaling".to_string(),
            status: format!("{:?}", data.time_scaling.read().await.status()),
            tasks_processed: metrics.tasks_processed,
            avg_processing_time: metrics.avg_processing_time,
            cpu_usage: metrics.cpu_usage,
            memory_usage: metrics.memory_usage,
            custom_metrics: metrics.custom_metrics,
        });
    }
    
    // Add memory management component metrics
    if let Ok(metrics) = data.memory_management.read().await.collect_metrics().await {
        component_metrics.insert("memory_management".to_string(), ComponentMetricsResponse {
            id: "memory_management".to_string(),
            status: format!("{:?}", data.memory_management.read().await.status()),
            tasks_processed: metrics.tasks_processed,
            avg_processing_time: metrics.avg_processing_time,
            cpu_usage: metrics.cpu_usage,
            memory_usage: metrics.memory_usage,
            custom_metrics: metrics.custom_metrics,
        });
    }
    
    // Add reasoning component metrics
    if let Ok(metrics) = data.reasoning.read().await.collect_metrics().await {
        component_metrics.insert("reasoning".to_string(), ComponentMetricsResponse {
            id: "reasoning".to_string(),
            status: format!("{:?}", data.reasoning.read().await.status()),
            tasks_processed: metrics.tasks_processed,
            avg_processing_time: metrics.avg_processing_time,
            cpu_usage: metrics.cpu_usage,
            memory_usage: metrics.memory_usage,
            custom_metrics: metrics.custom_metrics,
        });
    }
    
    // Add operational state component metrics
    if let Ok(metrics) = data.operational_state.read().await.collect_metrics().await {
        component_metrics.insert("operational_state".to_string(), ComponentMetricsResponse {
            id: "operational_state".to_string(),
            status: format!("{:?}", data.operational_state.read().await.status()),
            tasks_processed: metrics.tasks_processed,
            avg_processing_time: metrics.avg_processing_time,
            cpu_usage: metrics.cpu_usage,
            memory_usage: metrics.memory_usage,
            custom_metrics: metrics.custom_metrics,
        });
    }
    
    // Create response
    let response = SystemMetricsResponse {
        cpu_usage: system_metrics.total_cpu_usage,
        memory_usage: system_metrics.total_memory_usage,
        component_metrics,
        timestamp: Utc::now(),
    };
    
    HttpResponse::Ok().json(response)
}

/// Get information about all components
pub async fn get_components(data: web::Data<AppState>) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    let instances = orchestrator.get_all_instances();
    
    let mut components = Vec::new();
    
    // Add each component's info
    for (id, instance) in instances {
        let component_instance = instance.read().await;
        let component_info = ComponentInfoResponse {
            id: id.clone(),
            component_type: component_instance.component_type().to_string(),
            status: format!("{:?}", component_instance.status()),
            info: component_instance.get_info(),
        };
        components.push(component_info);
    }
    
    HttpResponse::Ok().json(web::Json(serde_json::json!({
        "success": true,
        "components": components
    })))
}