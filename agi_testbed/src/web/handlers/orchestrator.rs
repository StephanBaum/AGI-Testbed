use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use log::{info, error};
use uuid::Uuid;

use crate::web::server::AppState;
use crate::web::models::{MessageRequest, GenericResponse, ErrorResponse};
use crate::core::component::{ComponentMessage};

/// Get the orchestrator status
pub async fn get_status(data: web::Data<AppState>) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    let status = orchestrator.get_status();
    
    HttpResponse::Ok().json(web::Json(serde_json::json!({
        "success": true,
        "status": format!("{:?}", status.state),
        "active_components": status.active_components,
        "errors": status.errors,
        "warnings": status.warnings,
        "last_updated": status.last_updated
    })))
}

/// Start all component instances
pub async fn start_all(data: web::Data<AppState>) -> impl Responder {
    let mut orchestrator = data.orchestrator.write().await;
    
    match orchestrator.start_all().await {
        Ok(_) => {
            info!("Started all components successfully");
            HttpResponse::Ok().json(GenericResponse {
                success: true,
                message: "All components started successfully".to_string(),
                data: None,
            })
        },
        Err(e) => {
            error!("Failed to start components: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: format!("Failed to start components: {}", e),
                error_code: "COMPONENT_START_FAILED".to_string(),
            })
        }
    }
}

/// Stop all component instances
pub async fn stop_all(data: web::Data<AppState>) -> impl Responder {
    let mut orchestrator = data.orchestrator.write().await;
    
    match orchestrator.stop_all().await {
        Ok(_) => {
            info!("Stopped all components successfully");
            HttpResponse::Ok().json(GenericResponse {
                success: true,
                message: "All components stopped successfully".to_string(),
                data: None,
            })
        },
        Err(e) => {
            error!("Failed to stop components: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: format!("Failed to stop components: {}", e),
                error_code: "COMPONENT_STOP_FAILED".to_string(),
            })
        }
    }
}

/// Send a message to a component through the orchestrator
pub async fn send_message(
    data: web::Data<AppState>,
    request: web::Json<MessageRequest>,
) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    let message_id = Uuid::new_v4().to_string();
    
    // Create the message
    let message = ComponentMessage {
        id: message_id.clone(),
        sender: "web_api".to_string(),
        target: request.target.clone(),
        message_type: request.message_type.clone(),
        payload: request.payload.clone(),
        timestamp: Utc::now(),
        priority: request.priority.unwrap_or(5),
    };
    
    // If target is specified, send to that component
    // Otherwise, broadcast the message
    let result = if let Some(target) = &request.target {
        orchestrator.send_message(target, message).await
    } else {
        orchestrator.broadcast_message(message).await
    };
    
    match result {
        Ok(_) => {
            HttpResponse::Ok().json(GenericResponse {
                success: true,
                message: "Message sent successfully".to_string(),
                data: Some(serde_json::json!({
                    "message_id": message_id
                })),
            })
        },
        Err(e) => {
            error!("Failed to send message: {:?}", e);
            HttpResponse::InternalServerError().json(ErrorResponse {
                success: false,
                error: format!("Failed to send message: {}", e),
                error_code: "MESSAGE_SEND_FAILED".to_string(),
            })
        }
    }
}