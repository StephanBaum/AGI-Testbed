use actix_web::{web, HttpResponse, Responder, http::header};
use handlebars::Handlebars;
use serde_json::json;
use std::sync::Arc;
use log::error;

use crate::web::server::AppState;

/// Shared handlebars instance
lazy_static::lazy_static! {
    static ref HBS: Arc<Handlebars<'static>> = {
        let mut hbs = Handlebars::new();
        // Register templates
        if let Err(e) = hbs.register_templates_directory(".hbs", "./src/web/templates") {
            error!("Error registering Handlebars templates: {}", e);
        }
        Arc::new(hbs)
    };
}

/// Serve the index/home page
pub async fn index(data: web::Data<AppState>) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    let status = orchestrator.get_status();
    
    let context = json!({
        "title": "AGI Testbed",
        "system_status": format!("{:?}", status.state),
        "active_components": status.active_components,
        "version": env!("CARGO_PKG_VERSION"),
    });
    
    match HBS.render("index", &context) {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(e) => {
            error!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body(format!("Template error: {}", e))
        }
    }
}

/// Serve the dashboard page
pub async fn dashboard(data: web::Data<AppState>) -> impl Responder {
    let orchestrator = data.orchestrator.read().await;
    let status = orchestrator.get_status();
    
    // Get metrics if available
    let metrics = orchestrator.get_latest_metrics();
    
    let context = json!({
        "title": "Dashboard | AGI Testbed",
        "system_status": format!("{:?}", status.state),
        "active_components": status.active_components,
        "metrics": metrics,
        "errors": status.errors,
        "warnings": status.warnings,
    });
    
    match HBS.render("dashboard", &context) {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(e) => {
            error!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body(format!("Template error: {}", e))
        }
    }
}

/// Serve the time-scaling page
pub async fn time_scaling(data: web::Data<AppState>) -> impl Responder {
    let time_scaling = data.time_scaling.read().await;
    let info = time_scaling.get_info();
    
    let context = json!({
        "title": "Time Scaling | AGI Testbed",
        "component_id": time_scaling.id(),
        "status": format!("{:?}", time_scaling.status()),
        "processing_mode": info.get("processing_mode").unwrap_or(&json!("Unknown")),
        "scaling_factor": info.get("scaling_factor").unwrap_or(&json!(1.0)),
        "tasks_processed": info.get("tasks_processed").unwrap_or(&json!(0)),
    });
    
    match HBS.render("time_scaling", &context) {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(e) => {
            error!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body(format!("Template error: {}", e))
        }
    }
}

/// Serve the memory-management page
pub async fn memory_management(data: web::Data<AppState>) -> impl Responder {
    let memory = data.memory_management.read().await;
    let info = memory.get_info();
    
    // Create a task to get memory stats
    let task = crate::core::component::ComponentTask {
        id: uuid::Uuid::new_v4().to_string(),
        description: "Get memory stats for web UI".to_string(),
        parameters: json!({
            "operation": "get_stats"
        }),
        priority: 3,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    // Get memory stats
    let stats = data.orchestrator.read().await
        .submit_task("memory_management", task).await
        .unwrap_or(json!({"error": "Failed to get memory stats"}));
    
    let context = json!({
        "title": "Memory Management | AGI Testbed",
        "component_id": memory.id(),
        "status": format!("{:?}", memory.status()),
        "stats": stats,
        "memory_params": info.get("memory_params").unwrap_or(&json!({})),
    });
    
    match HBS.render("memory_management", &context) {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(e) => {
            error!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body(format!("Template error: {}", e))
        }
    }
}

/// Serve the reasoning page
pub async fn reasoning(data: web::Data<AppState>) -> impl Responder {
    let reasoning = data.reasoning.read().await;
    let info = reasoning.get_info();
    
    // Create a task to get reasoning stats
    let task = crate::core::component::ComponentTask {
        id: uuid::Uuid::new_v4().to_string(),
        description: "Get reasoning stats for web UI".to_string(),
        parameters: json!({
            "operation": "get_stats"
        }),
        priority: 3,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    // Get reasoning stats
    let stats = data.orchestrator.read().await
        .submit_task("reasoning", task).await
        .unwrap_or(json!({"error": "Failed to get reasoning stats"}));
    
    let context = json!({
        "title": "Reasoning | AGI Testbed",
        "component_id": reasoning.id(),
        "status": format!("{:?}", reasoning.status()),
        "stats": stats,
        "validation": info.get("validation").unwrap_or(&json!({})),
    });
    
    match HBS.render("reasoning", &context) {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(e) => {
            error!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body(format!("Template error: {}", e))
        }
    }
}

/// Serve the operational-state page
pub async fn operational_state(data: web::Data<AppState>) -> impl Responder {
    let operational = data.operational_state.read().await;
    let info = operational.get_info();
    
    // Create a task to get goals
    let goals_task = crate::core::component::ComponentTask {
        id: uuid::Uuid::new_v4().to_string(),
        description: "Get goals for web UI".to_string(),
        parameters: json!({
            "operation": "get_goals"
        }),
        priority: 3,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    // Get goals
    let goals = data.orchestrator.read().await
        .submit_task("operational_state", goals_task).await
        .unwrap_or(json!([{"error": "Failed to get goals"}]));
    
    // Create a task to get stats
    let stats_task = crate::core::component::ComponentTask {
        id: uuid::Uuid::new_v4().to_string(),
        description: "Get operational stats for web UI".to_string(),
        parameters: json!({
            "operation": "get_stats"
        }),
        priority: 3,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    // Get stats
    let stats = data.orchestrator.read().await
        .submit_task("operational_state", stats_task).await
        .unwrap_or(json!({"error": "Failed to get operational stats"}));
    
    let context = json!({
        "title": "Operational State | AGI Testbed",
        "component_id": operational.id(),
        "status": format!("{:?}", operational.status()),
        "current_mode": info.get("current_mode").unwrap_or(&json!("Unknown")),
        "autonomous_enabled": info.get("autonomous_enabled").unwrap_or(&json!(false)),
        "goals": goals,
        "stats": stats,
    });
    
    match HBS.render("operational_state", &context) {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(e) => {
            error!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body(format!("Template error: {}", e))
        }
    }
}

/// Serve the testbeds page
pub async fn testbeds(_data: web::Data<AppState>) -> impl Responder {
    let context = json!({
        "title": "Testbeds | AGI Testbed",
        "testbeds": [
            {
                "id": "testbed1",
                "name": "Time Scaling Testbed",
                "description": "Validate dynamic CPU/GPU time allocation",
                "status": "Ready"
            },
            {
                "id": "testbed2",
                "name": "Memory Allocation Testbed",
                "description": "Measure STM vs. LTM performance during high-load conditions",
                "status": "Ready"
            },
            {
                "id": "testbed3",
                "name": "Reasoning Validation Testbed",
                "description": "Validate knowledge integration & inference correctness",
                "status": "Ready"
            },
            {
                "id": "testbed4",
                "name": "Autonomous Operation Testbed",
                "description": "Assess system's ability to run indefinitely with changing goals",
                "status": "Ready"
            }
        ]
    });
    
    match HBS.render("testbeds", &context) {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(e) => {
            error!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body(format!("Template error: {}", e))
        }
    }
}

/// Serve the documentation page
pub async fn docs() -> impl Responder {
    let context = json!({
        "title": "Documentation | AGI Testbed",
    });
    
    match HBS.render("docs", &context) {
        Ok(body) => HttpResponse::Ok().content_type("text/html").body(body),
        Err(e) => {
            error!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body(format!("Template error: {}", e))
        }
    }
}

/// 404 Not Found handler
pub async fn not_found() -> impl Responder {
    let context = json!({
        "title": "Page Not Found | AGI Testbed",
    });
    
    match HBS.render("404", &context) {
        Ok(body) => HttpResponse::NotFound().content_type("text/html").body(body),
        Err(e) => {
            error!("Template rendering error: {}", e);
            HttpResponse::InternalServerError().body(format!("Template error: {}", e))
        }
    }
}