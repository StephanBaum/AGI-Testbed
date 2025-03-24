use log::{info, warn, error};
use std::sync::Arc;
use tokio::sync::RwLock;

mod core;
mod instances;
mod web;
mod utils;

use crate::core::orchestrator::Orchestrator;
use crate::instances::time_scaling::TimeScalingInstance;
use crate::instances::memory_management::MemoryManagementInstance;
use crate::instances::reasoning::ReasoningInstance;
use crate::instances::operational_state::OperationalStateInstance;
use crate::web::server::start_web_server;

#[tokio::main]
async fn main() {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    info!("Starting AGI Testbed...");
    
    // Create the core orchestrator
    let orchestrator = Arc::new(RwLock::new(Orchestrator::new()));
    info!("Core Orchestrator initialized");
    
    // Initialize component instances
    let time_scaling = Arc::new(RwLock::new(TimeScalingInstance::new()));
    let memory_management = Arc::new(RwLock::new(MemoryManagementInstance::new()));
    let reasoning = Arc::new(RwLock::new(ReasoningInstance::new()));
    let operational_state = Arc::new(RwLock::new(OperationalStateInstance::new()));
    
    info!("All component instances initialized");
    
    // Register instances with the orchestrator
    {
        let mut orch = orchestrator.write().await;
        orch.register_instance("time_scaling", time_scaling.clone());
        orch.register_instance("memory_management", memory_management.clone());
        orch.register_instance("reasoning", reasoning.clone());
        orch.register_instance("operational_state", operational_state.clone());
    }
    
    info!("All component instances registered with orchestrator");
    
    // Start the web interface
    info!("Starting web interface on http://localhost:8080");
    let web_server_handle = tokio::spawn(start_web_server(
        orchestrator.clone(),
        time_scaling.clone(),
        memory_management.clone(),
        reasoning.clone(),
        operational_state.clone(),
    ));
    
    // Run the system
    info!("AGI Testbed is now running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await.expect("Failed to listen for Ctrl+C");
    
    info!("Shutting down AGI Testbed...");
    
    // Graceful shutdown
    // Wait for web server to finish
    if let Err(e) = web_server_handle.await {
        error!("Error during web server shutdown: {:?}", e);
    }
    
    info!("AGI Testbed shutdown complete");
}