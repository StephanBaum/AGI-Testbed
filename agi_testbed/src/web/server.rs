use std::sync::Arc;
use tokio::sync::RwLock;
use actix_web::{web, App, HttpServer, middleware};
use actix_files as fs;
use log::{info, error};

use crate::core::orchestrator::Orchestrator;
use crate::instances::time_scaling::TimeScalingInstance;
use crate::instances::memory_management::MemoryManagementInstance;
use crate::instances::reasoning::ReasoningInstance;
use crate::instances::operational_state::OperationalStateInstance;
use crate::web::handlers;

/// Start the web server for the AGI testbed UI
pub async fn start_web_server(
    orchestrator: Arc<RwLock<Orchestrator>>,
    time_scaling: Arc<RwLock<TimeScalingInstance>>,
    memory_management: Arc<RwLock<MemoryManagementInstance>>,
    reasoning: Arc<RwLock<ReasoningInstance>>,
    operational_state: Arc<RwLock<OperationalStateInstance>>,
) -> std::io::Result<()> {
    info!("Starting web server on http://localhost:8080");
    
    // Create shared application state
    let app_state = web::Data::new(AppState {
        orchestrator: orchestrator.clone(),
        time_scaling: time_scaling.clone(),
        memory_management: memory_management.clone(),
        reasoning: reasoning.clone(),
        operational_state: operational_state.clone(),
    });
    
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .app_data(app_state.clone())
            // Static files
            .service(fs::Files::new("/static", "./src/web/static").show_files_listing(false))
            // API routes
            .service(
                web::scope("/api")
                    // System APIs
                    .route("/system/status", web::get().to(handlers::system::get_system_status))
                    .route("/system/metrics", web::get().to(handlers::system::get_system_metrics))
                    .route("/system/components", web::get().to(handlers::system::get_components))
                    
                    // Orchestrator APIs
                    .route("/orchestrator/status", web::get().to(handlers::orchestrator::get_status))
                    .route("/orchestrator/start", web::post().to(handlers::orchestrator::start_all))
                    .route("/orchestrator/stop", web::post().to(handlers::orchestrator::stop_all))
                    .route("/orchestrator/message", web::post().to(handlers::orchestrator::send_message))
                    
                    // Time Scaling APIs
                    .route("/time-scaling/status", web::get().to(handlers::time_scaling::get_status))
                    .route("/time-scaling/mode", web::get().to(handlers::time_scaling::get_mode))
                    .route("/time-scaling/mode", web::post().to(handlers::time_scaling::set_mode))
                    .route("/time-scaling/metrics", web::get().to(handlers::time_scaling::get_metrics))
                    .route("/time-scaling/tasks", web::post().to(handlers::time_scaling::submit_task))
                    .route("/time-scaling/stats", web::get().to(handlers::time_scaling::get_stats))
                    
                    // Memory Management APIs
                    .route("/memory/status", web::get().to(handlers::memory::get_status))
                    .route("/memory/stats", web::get().to(handlers::memory::get_stats))
                    .route("/memory/store", web::post().to(handlers::memory::store_item))
                    .route("/memory/retrieve", web::post().to(handlers::memory::retrieve_item))
                    .route("/memory/update", web::post().to(handlers::memory::update_item))
                    .route("/memory/delete", web::post().to(handlers::memory::delete_item))
                    .route("/memory/consolidate", web::post().to(handlers::memory::consolidate))
                    .route("/memory/allocation", web::post().to(handlers::memory::update_allocation))
                    
                    // Reasoning APIs
                    .route("/reasoning/status", web::get().to(handlers::reasoning::get_status))
                    .route("/reasoning/stats", web::get().to(handlers::reasoning::get_stats))
                    .route("/reasoning/node", web::post().to(handlers::reasoning::add_node))
                    .route("/reasoning/edge", web::post().to(handlers::reasoning::add_edge))
                    .route("/reasoning/rule", web::post().to(handlers::reasoning::add_rule))
                    .route("/reasoning/infer", web::post().to(handlers::reasoning::run_inference))
                    .route("/reasoning/query", web::post().to(handlers::reasoning::query_knowledge))
                    
                    // Operational State APIs
                    .route("/operational/status", web::get().to(handlers::operational::get_status))
                    .route("/operational/mode", web::get().to(handlers::operational::get_mode))
                    .route("/operational/mode", web::post().to(handlers::operational::switch_mode))
                    .route("/operational/stats", web::get().to(handlers::operational::get_stats))
                    .route("/operational/tasks", web::get().to(handlers::operational::get_tasks))
                    .route("/operational/goals", web::get().to(handlers::operational::get_goals))
                    .route("/operational/goal", web::post().to(handlers::operational::create_goal))
                    .route("/operational/task", web::post().to(handlers::operational::submit_task))
                    .route("/operational/recovery", web::post().to(handlers::operational::handle_recovery))
                    
                    // Testbed APIs
                    .route("/testbed/run", web::post().to(handlers::testbed::run_testbed))
                    .route("/testbed/status", web::get().to(handlers::testbed::get_testbed_status))
                    .route("/testbed/results", web::get().to(handlers::testbed::get_testbed_results))
            )
            // Page routes
            .route("/", web::get().to(handlers::pages::index))
            .route("/dashboard", web::get().to(handlers::pages::dashboard))
            .route("/time-scaling", web::get().to(handlers::pages::time_scaling))
            .route("/memory-management", web::get().to(handlers::pages::memory_management))
            .route("/reasoning", web::get().to(handlers::pages::reasoning))
            .route("/operational-state", web::get().to(handlers::pages::operational_state))
            .route("/testbeds", web::get().to(handlers::pages::testbeds))
            .route("/docs", web::get().to(handlers::pages::docs))
            // Default route for 404
            .default_service(web::get().to(handlers::pages::not_found))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

/// Shared application state for web handlers
pub struct AppState {
    pub orchestrator: Arc<RwLock<Orchestrator>>,
    pub time_scaling: Arc<RwLock<TimeScalingInstance>>,
    pub memory_management: Arc<RwLock<MemoryManagementInstance>>,
    pub reasoning: Arc<RwLock<ReasoningInstance>>,
    pub operational_state: Arc<RwLock<OperationalStateInstance>>,
}