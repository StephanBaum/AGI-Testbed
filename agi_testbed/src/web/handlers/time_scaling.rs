use actix_web::{web, HttpResponse, Responder};
use log::{info, error};
use uuid::Uuid;
use serde_json::json;

use crate::web::server::AppState;
use crate::web::models::{TimeScalingModeRequest, TaskRequest, GenericResponse, ErrorResponse};
use crate::core::component::{ComponentTask};
use crate::instances::time_scaling::ProcessingMode;

/// Get the current status of the time scaling component
pub async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let time_scaling = data.time_scaling.read().await;
    
    HttpResponse::Ok().json(web::Json(json!({
        "success": true,
        "status": format!("{:?}", time_scaling.status()),
        "component_type": time_scaling.component_type(),
        "id": time_scaling.id()
    })))
}

/// Get the current processing mode of the time scaling component
pub async fn get_mode(data: web::Data<AppState>) -> impl Responder {
    let time_scaling = data.time_scaling.read().await;
    
    // Use the component's get_info method to get the current mode
    let info = time_scaling.get_info();
    
    HttpResponse::Ok().json(web::Json(json!({
        "success": true,
        "mode": info.get("processing_mode").unwrap_or(&json!("unknown")),
        "scaling_factor": info.get("scaling_factor").unwrap_or(&json!(1.0))
    })))
}

/// Set the processing mode of the time scaling component
pub async fn set_mode(
    data: web::Data<AppState>,
    request: web::Json<TimeScalingModeRequest>,
) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    
    // Create a task to change the mode
    let task = ComponentTask {
        id: Uuid::new_v4().to_string(),
        description: format!("Set time scaling mode to {}", request.mode),
        parameters: json!({
            "operation": "change_mode",
            "mode": request.mode,
            "factor": request.factor
        }),
        priority: 8,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    // Submit task to the time scaling component
    match orchestrator.submit_task("time_scaling", task).await {
        Ok(result) => {
            info!("Set time scaling mode successfully: {:?}", result);
            HttpResponse::Ok().json(GenericResponse {
                success: true,
                message: format!("Set time scaling mode to {}", request.mode),
                data: Some(result),
            })
        },
        Err(e) => {
            error!("Failed to set time scaling mode: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: format!("Failed to set time scaling mode: {}", e),
                error_code: "MODE_CHANGE_FAILED".to_string(),
            })
        }
    }
}

/// Get the current metrics of the time scaling component
pub async fn get_metrics(data: web::Data<AppState>) -> impl Responder {
    let time_scaling = data.time_scaling.read().await;
    
    match time_scaling.collect_metrics().await {
        Ok(metrics) => {
            HttpResponse::Ok().json(web::Json(json!({
                "success": true,
                "metrics": {
                    "cpu_usage": metrics.cpu_usage,
                    "memory_usage": metrics.memory_usage,
                    "tasks_processed": metrics.tasks_processed,
                    "avg_processing_time": metrics.avg_processing_time,
                    "custom_metrics": metrics.custom_metrics
                },
                "timestamp": chrono::Utc::now()
            })))
        },
        Err(e) => {
            error!("Failed to get time scaling metrics: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: format!("Failed to get time scaling metrics: {}", e),
                error_code: "METRICS_FETCH_FAILED".to_string(),
            })
        }
    }
}

/// Submit a task to the time scaling component
pub async fn submit_task(
    data: web::Data<AppState>,
    request: web::Json<TaskRequest>,
) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    
    // Create a task
    let task = ComponentTask {
        id: Uuid::new_v4().to_string(),
        description: request.description.clone(),
        parameters: json!({
            "operation": request.operation,
            ...request.parameters
        }),
        priority: request.priority.unwrap_or(5),
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    // Submit task to the time scaling component
    match orchestrator.submit_task("time_scaling", task).await {
        Ok(result) => {
            info!("Task submitted to time scaling component successfully");
            HttpResponse::Ok().json(GenericResponse {
                success: true,
                message: "Task submitted successfully".to_string(),
                data: Some(result),
            })
        },
        Err(e) => {
            error!("Failed to submit task to time scaling component: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: format!("Failed to submit task: {}", e),
                error_code: "TASK_SUBMISSION_FAILED".to_string(),
            })
        }
    }
}

/// Get stats from the time scaling component
pub async fn get_stats(data: web::Data<AppState>) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    
    // Create a task to get stats
    let task = ComponentTask {
        id: Uuid::new_v4().to_string(),
        description: "Get time scaling statistics".to_string(),
        parameters: json!({
            "operation": "get_stats"
        }),
        priority: 3,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    // Submit task to the time scaling component
    match orchestrator.submit_task("time_scaling", task).await {
        Ok(result) => {
            HttpResponse::Ok().json(GenericResponse {
                success: true,
                message: "Retrieved time scaling statistics successfully".to_string(),
                data: Some(result),
            })
        },
        Err(e) => {
            error!("Failed to get time scaling stats: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: format!("Failed to get time scaling statistics: {}", e),
                error_code: "STATS_FETCH_FAILED".to_string(),
            })
        }
    }
}