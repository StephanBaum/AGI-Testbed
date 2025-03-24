//! Web handlers for the Operational State instance
//!
//! This module provides handlers for the operational state API endpoints.

use actix_web::{web, HttpResponse, Responder};
use serde::{Serialize, Deserialize};
use serde_json::json;
use chrono::Utc;
use log::error;

use crate::web::server::AppState;
use crate::instances::operational_state::models::{OperationalMode, GoalStatus, RecoveryState};

/// Get current status of the operational state instance
pub async fn get_status(app_state: web::Data<AppState>) -> impl Responder {
    let instance = app_state.operational_state.read().await;
    
    let status = instance.status();
    let mode = instance.mode.read().await;
    
    HttpResponse::Ok().json(json!({
        "status": format!("{}", status),
        "mode": instance.get_mode_name(&mode),
        "autonomous_enabled": instance.autonomous_settings.enabled,
        "running_tasks": instance.running_tasks.lock().await.len(),
        "queued_tasks": instance.task_queue.lock().await.len(),
        "active_goals": instance.goals.read().await.iter()
            .filter(|g| g.status == GoalStatus::Pending || g.status == GoalStatus::InProgress)
            .count(),
        "recovery_state": format!("{:?}", instance.recovery_data.read().await.recovery_state),
    }))
}

/// Get current operational mode
pub async fn get_mode(app_state: web::Data<AppState>) -> impl Responder {
    let instance = app_state.operational_state.read().await;
    let mode = instance.mode.read().await;
    
    let mode_name = instance.get_mode_name(&mode);
    let mode_config = instance.get_mode_config(&mode);
    
    HttpResponse::Ok().json(json!({
        "current_mode": mode_name,
        "mode_details": mode_config,
        "available_modes": instance.available_modes,
    }))
}

/// Switch operational mode
#[derive(Deserialize)]
pub struct SwitchModeRequest {
    /// The mode to switch to
    pub mode: String,
}

pub async fn switch_mode(
    app_state: web::Data<AppState>,
    req: web::Json<SwitchModeRequest>,
) -> impl Responder {
    let instance = app_state.operational_state.read().await;
    
    let mode = match req.mode.as_str() {
        "problem_solving" => OperationalMode::ProblemSolving,
        "exploration" => OperationalMode::Exploration,
        "consolidation" => OperationalMode::Consolidation,
        "planning" => OperationalMode::Planning,
        _ => OperationalMode::Custom(req.mode.clone()),
    };
    
    match instance.switch_mode(mode.clone()).await {
        Ok(_) => {
            let new_mode = instance.mode.read().await;
            
            HttpResponse::Ok().json(json!({
                "success": true,
                "current_mode": instance.get_mode_name(&new_mode),
                "message": format!("Switched to {} mode", instance.get_mode_name(&new_mode)),
            }))
        },
        Err(e) => {
            error!("Error switching mode: {:?}", e);
            
            HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("{}", e),
            }))
        }
    }
}

/// Get operational stats
pub async fn get_stats(app_state: web::Data<AppState>) -> impl Responder {
    let instance = app_state.operational_state.read().await;
    
    match instance.get_stats().await {
        stats => {
            // Add resource stats
            let resource_stats = instance.get_resource_stats().await;
            
            HttpResponse::Ok().json(json!({
                "stats": stats,
                "resources": resource_stats,
                "current_mode": instance.get_mode_name(&instance.mode.read().await),
                "task_count": {
                    "queued": instance.task_queue.lock().await.len(),
                    "running": instance.running_tasks.lock().await.len(),
                    "scheduled": instance.scheduled_tasks.lock().await.len(),
                },
                "timestamp": Utc::now(),
            }))
        }
    }
}

/// Get current tasks
pub async fn get_tasks(app_state: web::Data<AppState>) -> impl Responder {
    let instance = app_state.operational_state.read().await;
    
    let running_tasks = instance.running_tasks.lock().await;
    let queued_tasks = instance.task_queue.lock().await;
    let scheduled_tasks = instance.scheduled_tasks.lock().await;
    
    // Get the last 100 completed tasks
    let history = instance.get_task_history(100).await;
    
    HttpResponse::Ok().json(json!({
        "running": running_tasks.values().collect::<Vec<_>>(),
        "queued": queued_tasks.iter().collect::<Vec<_>>(),
        "scheduled": scheduled_tasks.iter().collect::<Vec<_>>(),
        "history": history,
    }))
}

/// Get goals
pub async fn get_goals(app_state: web::Data<AppState>) -> impl Responder {
    let instance = app_state.operational_state.read().await;
    
    let goals = instance.get_goals().await;
    
    HttpResponse::Ok().json(json!({
        "goals": goals,
        "active_count": goals.iter()
            .filter(|g| g.status == GoalStatus::Pending || g.status == GoalStatus::InProgress)
            .count(),
        "completed_count": goals.iter()
            .filter(|g| g.status == GoalStatus::Completed)
            .count(),
        "stats": instance.stats.read().await.goal_stats,
    }))
}

/// Create goal request
#[derive(Deserialize)]
pub struct CreateGoalRequest {
    /// Goal description
    pub description: String,
    /// Goal priority (optional, default 5)
    pub priority: Option<u8>,
    /// Target descriptions (optional)
    pub targets: Option<Vec<CreateTargetRequest>>,
}

/// Create target request
#[derive(Deserialize)]
pub struct CreateTargetRequest {
    /// Target description
    pub description: String,
    /// Target criteria (optional)
    pub criteria: Option<serde_json::Value>,
}

/// Create a new goal
pub async fn create_goal(
    app_state: web::Data<AppState>,
    req: web::Json<CreateGoalRequest>,
) -> impl Responder {
    let instance = app_state.operational_state.read().await;
    
    let priority = req.priority.unwrap_or(5);
    
    match instance.create_goal(&req.description, priority).await {
        Ok(goal_id) => {
            // Add targets if provided
            if let Some(targets) = &req.targets {
                for target in targets {
                    let criteria = target.criteria.clone().unwrap_or(json!({}));
                    
                    if let Err(e) = instance.add_goal_target(&goal_id, &target.description, criteria).await {
                        error!("Error adding target to goal {}: {:?}", goal_id, e);
                    }
                }
            }
            
            HttpResponse::Ok().json(json!({
                "success": true,
                "goal_id": goal_id,
                "message": format!("Created goal: {}", req.description),
            }))
        },
        Err(e) => {
            error!("Error creating goal: {:?}", e);
            
            HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("{}", e),
            }))
        }
    }
}

/// Submit task request
#[derive(Deserialize)]
pub struct SubmitTaskRequest {
    /// Task description
    pub description: String,
    /// Task category (optional, default "default")
    pub category: Option<String>,
    /// Task priority (optional, default 5)
    pub priority: Option<u8>,
    /// Task parameters (optional)
    pub parameters: Option<serde_json::Value>,
    /// Required resources (optional)
    pub resources: Option<HashMap<String, f64>>,
    /// Estimated duration in seconds (optional, default 60.0)
    pub duration: Option<f64>,
    /// Schedule time (optional, if provided the task will be scheduled for future execution)
    pub schedule_time: Option<String>,
    /// Recurrence pattern (optional)
    pub recurrence: Option<String>,
}

/// Submit a task
pub async fn submit_task(
    app_state: web::Data<AppState>,
    req: web::Json<SubmitTaskRequest>,
) -> impl Responder {
    use std::collections::HashMap;
    use uuid::Uuid;
    
    let instance = app_state.operational_state.read().await;
    
    let task = crate::core::component::ComponentTask {
        id: Uuid::new_v4().to_string(),
        description: req.description.clone(),
        parameters: req.parameters.clone().unwrap_or(json!({})),
        priority: req.priority.unwrap_or(5),
        deadline: None,
        created_at: Utc::now(),
    };
    
    let prioritized = crate::instances::operational_state::models::PrioritizedTask {
        task: task.clone(),
        category: req.category.clone().unwrap_or("default".to_string()),
        priority: req.priority.unwrap_or(5),
        required_resources: req.resources.clone().unwrap_or_else(HashMap::new),
        estimated_duration: req.duration.unwrap_or(60.0),
        is_system_task: false,
        interruptible: true,
        queued_at: Utc::now(),
    };
    
    // Check if the task should be scheduled for future execution
    if let Some(schedule_time) = &req.schedule_time {
        match chrono::DateTime::parse_from_rfc3339(schedule_time) {
            Ok(time) => {
                let time_utc = time.with_timezone(&Utc);
                
                // Parse recurrence if any
                let recurrence = if let Some(rec_str) = &req.recurrence {
                    match rec_str.as_str() {
                        "hourly" => Some(crate::instances::operational_state::models::RecurrencePattern::EveryHours(1)),
                        "daily" => Some(crate::instances::operational_state::models::RecurrencePattern::EveryDays(1)),
                        "weekly" => Some(crate::instances::operational_state::models::RecurrencePattern::WeeklyOnDays(vec![0])),
                        "monthly" => Some(crate::instances::operational_state::models::RecurrencePattern::MonthlyOnDays(vec![1])),
                        _ => None,
                    }
                } else {
                    None
                };
                
                match instance.schedule_task(prioritized, time_utc, recurrence).await {
                    Ok(_) => {
                        HttpResponse::Ok().json(json!({
                            "success": true,
                            "task_id": task.id,
                            "message": format!("Scheduled task for {}", time_utc),
                            "scheduled_time": time_utc,
                        }))
                    },
                    Err(e) => {
                        error!("Error scheduling task: {:?}", e);
                        
                        HttpResponse::BadRequest().json(json!({
                            "success": false,
                            "error": format!("{}", e),
                        }))
                    }
                }
            },
            Err(e) => {
                error!("Error parsing schedule time: {:?}", e);
                
                HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "error": format!("Invalid schedule time: {}", e),
                }))
            }
        }
    } else {
        // Queue the task immediately
        match instance.enqueue_task(prioritized).await {
            Ok(_) => {
                HttpResponse::Ok().json(json!({
                    "success": true,
                    "task_id": task.id,
                    "message": format!("Queued task: {}", req.description),
                }))
            },
            Err(e) => {
                error!("Error queueing task: {:?}", e);
                
                HttpResponse::BadRequest().json(json!({
                    "success": false,
                    "error": format!("{}", e),
                }))
            }
        }
    }
}

/// Handle recovery request
#[derive(Deserialize)]
pub struct RecoveryRequest {
    /// Recovery action (start, abort, reset)
    pub action: String,
}

/// Handle recovery
pub async fn handle_recovery(
    app_state: web::Data<AppState>,
    req: web::Json<RecoveryRequest>,
) -> impl Responder {
    let instance = app_state.operational_state.read().await;
    
    match req.action.as_str() {
        "start" => {
            match instance.recover_from_interruption().await {
                Ok(_) => {
                    HttpResponse::Ok().json(json!({
                        "success": true,
                        "message": "Recovery process started",
                        "recovery_state": format!("{:?}", instance.recovery_data.read().await.recovery_state),
                    }))
                },
                Err(e) => {
                    error!("Error starting recovery: {:?}", e);
                    
                    HttpResponse::BadRequest().json(json!({
                        "success": false,
                        "error": format!("{}", e),
                    }))
                }
            }
        },
        "abort" => {
            // Mark recovery as failed
            {
                let mut recovery_data = instance.recovery_data.write().await;
                recovery_data.recovery_state = RecoveryState::Failed;
            }
            
            HttpResponse::Ok().json(json!({
                "success": true,
                "message": "Recovery process aborted",
                "recovery_state": "Failed",
            }))
        },
        "reset" => {
            // Reset recovery state
            {
                let mut recovery_data = instance.recovery_data.write().await;
                recovery_data.recovery_state = RecoveryState::None;
                recovery_data.interrupted_tasks.clear();
                recovery_data.pending_tasks.clear();
                recovery_data.interrupted_goals.clear();
            }
            
            HttpResponse::Ok().json(json!({
                "success": true,
                "message": "Recovery state reset",
                "recovery_state": "None",
            }))
        },
        _ => {
            HttpResponse::BadRequest().json(json!({
                "success": false,
                "error": format!("Invalid recovery action: {}", req.action),
            }))
        }
    }
}