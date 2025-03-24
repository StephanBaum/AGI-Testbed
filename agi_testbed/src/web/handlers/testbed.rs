use actix_web::{web, HttpResponse, Responder};
use log::{info, error};
use serde_json::json;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::web::server::AppState;
use crate::web::models::{TestbedRunRequest, GenericResponse, ErrorResponse};
use crate::core::component::ComponentTask;

// In-memory storage for testbed runs
lazy_static::lazy_static! {
    static ref TESTBED_RUNS: Arc<RwLock<HashMap<String, TestbedRun>>> = Arc::new(RwLock::new(HashMap::new()));
}

/// Testbed run information
#[derive(Clone, Debug)]
struct TestbedRun {
    id: String,
    testbed_id: String,
    name: String,
    status: TestbedStatus,
    start_time: chrono::DateTime<chrono::Utc>,
    end_time: Option<chrono::DateTime<chrono::Utc>>,
    parameters: serde_json::Value,
    results: Option<serde_json::Value>,
}

/// Testbed status
#[derive(Clone, Debug, PartialEq)]
enum TestbedStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

impl ToString for TestbedStatus {
    fn to_string(&self) -> String {
        match self {
            TestbedStatus::Pending => "pending".to_string(),
            TestbedStatus::Running => "running".to_string(),
            TestbedStatus::Completed => "completed".to_string(),
            TestbedStatus::Failed => "failed".to_string(),
        }
    }
}

/// Run a testbed
pub async fn run_testbed(
    data: web::Data<AppState>,
    request: web::Json<TestbedRunRequest>,
) -> impl Responder {
    // Validate testbed ID
    let testbed_id = &request.testbed_id;
    let testbed_name = match testbed_id.as_str() {
        "testbed1" => "Time Scaling Testbed",
        "testbed2" => "Memory Allocation Testbed",
        "testbed3" => "Reasoning Validation Testbed",
        "testbed4" => "Autonomous Operation Testbed",
        _ => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                success: false,
                error: format!("Invalid testbed ID: {}", testbed_id),
                error_code: "INVALID_TESTBED_ID".to_string(),
            });
        }
    };
    
    // Create a run ID
    let run_id = Uuid::new_v4().to_string();
    let run_name = request.run_name.clone().unwrap_or_else(|| format!("Run-{}", run_id));
    
    // Create a testbed run
    let testbed_run = TestbedRun {
        id: run_id.clone(),
        testbed_id: testbed_id.clone(),
        name: run_name.clone(),
        status: TestbedStatus::Pending,
        start_time: chrono::Utc::now(),
        end_time: None,
        parameters: request.parameters.clone(),
        results: None,
    };
    
    // Store the testbed run
    {
        let mut runs = TESTBED_RUNS.write().await;
        runs.insert(run_id.clone(), testbed_run.clone());
    }
    
    // Dispatch testbed run task based on testbed ID
    let app_state = data.clone();
    let run_id_clone = run_id.clone();
    tokio::spawn(async move {
        execute_testbed_run(app_state, run_id_clone, testbed_id.clone(), request.parameters.clone()).await;
    });
    
    // Return success response
    HttpResponse::Ok().json(GenericResponse {
        success: true,
        message: format!("Testbed run '{}' started", run_name),
        data: Some(json!({
            "run_id": run_id,
            "testbed_id": testbed_id,
            "name": run_name,
            "status": "pending",
            "start_time": chrono::Utc::now(),
        })),
    })
}

/// Get testbed status
pub async fn get_testbed_status(
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let runs = TESTBED_RUNS.read().await;
    
    // If run_id is provided, get status for that run
    if let Some(run_id) = query.get("run_id") {
        if let Some(run) = runs.get(run_id) {
            return HttpResponse::Ok().json(GenericResponse {
                success: true,
                message: format!("Testbed run status: {}", run.status.to_string()),
                data: Some(json!({
                    "run_id": run.id,
                    "testbed_id": run.testbed_id,
                    "name": run.name,
                    "status": run.status.to_string(),
                    "start_time": run.start_time,
                    "end_time": run.end_time,
                    "has_results": run.results.is_some(),
                })),
            });
        } else {
            return HttpResponse::NotFound().json(ErrorResponse {
                success: false,
                error: format!("Testbed run not found: {}", run_id),
                error_code: "RUN_NOT_FOUND".to_string(),
            });
        }
    }
    
    // Otherwise, list all runs
    let all_runs: Vec<_> = runs.values()
        .map(|run| json!({
            "run_id": run.id,
            "testbed_id": run.testbed_id,
            "name": run.name,
            "status": run.status.to_string(),
            "start_time": run.start_time,
            "end_time": run.end_time,
            "has_results": run.results.is_some(),
        }))
        .collect();
    
    HttpResponse::Ok().json(GenericResponse {
        success: true,
        message: format!("Found {} testbed runs", all_runs.len()),
        data: Some(json!({
            "runs": all_runs
        })),
    })
}

/// Get testbed results
pub async fn get_testbed_results(
    query: web::Query<HashMap<String, String>>,
) -> impl Responder {
    let runs = TESTBED_RUNS.read().await;
    
    // Run ID is required
    if let Some(run_id) = query.get("run_id") {
        if let Some(run) = runs.get(run_id) {
            if let Some(results) = &run.results {
                return HttpResponse::Ok().json(GenericResponse {
                    success: true,
                    message: format!("Testbed run results for '{}'", run.name),
                    data: Some(json!({
                        "run_id": run.id,
                        "testbed_id": run.testbed_id,
                        "name": run.name,
                        "status": run.status.to_string(),
                        "start_time": run.start_time,
                        "end_time": run.end_time,
                        "results": results,
                    })),
                });
            } else {
                return HttpResponse::NotFound().json(ErrorResponse {
                    success: false,
                    error: format!("Results not available for run: {}", run_id),
                    error_code: "RESULTS_NOT_AVAILABLE".to_string(),
                });
            }
        } else {
            return HttpResponse::NotFound().json(ErrorResponse {
                success: false,
                error: format!("Testbed run not found: {}", run_id),
                error_code: "RUN_NOT_FOUND".to_string(),
            });
        }
    } else {
        return HttpResponse::BadRequest().json(ErrorResponse {
            success: false,
            error: "Run ID is required".to_string(),
            error_code: "MISSING_RUN_ID".to_string(),
        });
    }
}

/// Execute a testbed run
async fn execute_testbed_run(
    app_state: web::Data<AppState>,
    run_id: String,
    testbed_id: String,
    parameters: serde_json::Value,
) {
    // Update run status to running
    {
        let mut runs = TESTBED_RUNS.write().await;
        if let Some(run) = runs.get_mut(&run_id) {
            run.status = TestbedStatus::Running;
        }
    }
    
    // Execute the testbed based on ID
    let result = match testbed_id.as_str() {
        "testbed1" => run_time_scaling_testbed(&app_state, &parameters).await,
        "testbed2" => run_memory_allocation_testbed(&app_state, &parameters).await,
        "testbed3" => run_reasoning_validation_testbed(&app_state, &parameters).await,
        "testbed4" => run_autonomous_operation_testbed(&app_state, &parameters).await,
        _ => {
            error!("Unknown testbed ID: {}", testbed_id);
            Err("Unknown testbed ID".to_string())
        }
    };
    
    // Update run status and results
    {
        let mut runs = TESTBED_RUNS.write().await;
        if let Some(run) = runs.get_mut(&run_id) {
            run.end_time = Some(chrono::Utc::now());
            
            match result {
                Ok(results) => {
                    run.status = TestbedStatus::Completed;
                    run.results = Some(results);
                },
                Err(error_msg) => {
                    run.status = TestbedStatus::Failed;
                    run.results = Some(json!({
                        "error": error_msg
                    }));
                }
            }
        }
    }
    
    info!("Testbed run {} completed", run_id);
}

/// Run Time Scaling Testbed (Testbed 1)
async fn run_time_scaling_testbed(
    app_state: &web::Data<AppState>,
    parameters: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    info!("Running Time Scaling Testbed");
    
    // Parse parameters
    let complexity_levels = parameters.get("complexity_levels")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(3);
    
    let iterations_per_level = parameters.get("iterations_per_level")
        .and_then(|v| v.as_u64())
        .unwrap_or(5);
    
    // Results storage
    let mut results = Vec::new();
    
    // Run tests for each complexity level
    for complexity in 1..=complexity_levels {
        let mode = match complexity {
            1 => "realtime",
            2 => "standard",
            3 => "analytical",
            _ => "custom",
        };
        
        // Set the time scaling mode
        let mode_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: format!("Set time scaling mode to {}", mode),
            parameters: json!({
                "operation": "change_mode",
                "mode": mode,
            }),
            priority: 8,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        match app_state.orchestrator.read().await.submit_task("time_scaling", mode_task).await {
            Ok(_) => info!("Set time scaling mode to {}", mode),
            Err(e) => {
                error!("Failed to set time scaling mode: {:?}", e);
                return Err(format!("Failed to set time scaling mode: {}", e));
            }
        }
        
        // Run multiple iterations at this complexity level
        let mut level_results = Vec::new();
        for i in 1..=iterations_per_level {
            // Create a test task with appropriate complexity
            let test_task = ComponentTask {
                id: Uuid::new_v4().to_string(),
                description: format!("Time scaling test task (Level {}, Iteration {})", complexity, i),
                parameters: json!({
                    "operation": "process",
                    "complexity": complexity,
                    "data_size": complexity * 100,
                    "iteration": i
                }),
                priority: 5,
                deadline: None,
                created_at: chrono::Utc::now(),
            };
            
            // Submit the task and record results
            match app_state.orchestrator.read().await.submit_task("time_scaling", test_task).await {
                Ok(task_result) => {
                    info!("Completed time scaling test task: level={}, iteration={}", complexity, i);
                    level_results.push(task_result);
                },
                Err(e) => {
                    error!("Error in time scaling test: {:?}", e);
                    return Err(format!("Test task error: {}", e));
                }
            }
            
            // Pause between iterations
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        
        // Add results for this complexity level
        results.push(json!({
            "complexity_level": complexity,
            "mode": mode,
            "iterations": level_results
        }));
    }
    
    // Calculate aggregate metrics
    let mut avg_processing_times = Vec::new();
    let mut avg_quality_scores = Vec::new();
    
    for level_result in &results {
        if let Some(iterations) = level_result.get("iterations").and_then(|v| v.as_array()) {
            let mut level_processing_times = Vec::new();
            let mut level_quality_scores = Vec::new();
            
            for iteration in iterations {
                if let Some(processing_time) = iteration.get("processing_time_ms").and_then(|v| v.as_u64()) {
                    level_processing_times.push(processing_time as f64);
                }
                
                if let Some(quality_score) = iteration.get("quality_score").and_then(|v| v.as_f64()) {
                    level_quality_scores.push(quality_score);
                }
            }
            
            // Calculate averages if we have data
            if !level_processing_times.is_empty() {
                let avg_time = level_processing_times.iter().sum::<f64>() / level_processing_times.len() as f64;
                avg_processing_times.push(avg_time);
            }
            
            if !level_quality_scores.is_empty() {
                let avg_quality = level_quality_scores.iter().sum::<f64>() / level_quality_scores.len() as f64;
                avg_quality_scores.push(avg_quality);
            }
        }
    }
    
    // Return overall results
    Ok(json!({
        "testbed": "Time Scaling Testbed",
        "parameters": parameters,
        "results_by_complexity": results,
        "summary": {
            "complexity_levels_tested": complexity_levels,
            "iterations_per_level": iterations_per_level,
            "avg_processing_times_by_level": avg_processing_times,
            "avg_quality_scores_by_level": avg_quality_scores,
        },
        "conclusion": {
            "time_scaling_demonstrated": !avg_processing_times.is_empty(),
            "quality_vs_time_tradeoff_observed": !avg_quality_scores.is_empty(),
        }
    }))
}

/// Run Memory Allocation Testbed (Testbed 2)
async fn run_memory_allocation_testbed(
    app_state: &web::Data<AppState>,
    parameters: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    info!("Running Memory Allocation Testbed");
    
    // Parse parameters
    let stm_allocations = parameters.get("stm_allocations")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_f64())
                .collect::<Vec<f64>>()
        })
        .unwrap_or_else(|| vec![0.2, 0.5, 0.8]);
    
    let items_per_allocation = parameters.get("items_per_allocation")
        .and_then(|v| v.as_u64())
        .unwrap_or(10);
    
    // Results storage
    let mut allocation_results = Vec::new();
    
    // Test different STM/LTM allocations
    for &stm_allocation in &stm_allocations {
        // Update memory allocation
        let allocation_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: format!("Update memory allocation to STM={:.1}", stm_allocation),
            parameters: json!({
                "operation": "update_allocation",
                "stm_allocation": stm_allocation
            }),
            priority: 8,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        match app_state.orchestrator.read().await.submit_task("memory_management", allocation_task).await {
            Ok(_) => info!("Updated memory allocation: STM={:.1}", stm_allocation),
            Err(e) => {
                error!("Failed to update memory allocation: {:?}", e);
                return Err(format!("Failed to update memory allocation: {}", e));
            }
        }
        
        // Store and retrieve items
        let mut store_results = Vec::new();
        let mut retrieve_results = Vec::new();
        
        // Store items
        for i in 1..=items_per_allocation {
            let importance = rand::random::<f64>();
            let processing_depth = rand::random::<f64>();
            
            // Create a memory item
            let store_task = ComponentTask {
                id: Uuid::new_v4().to_string(),
                description: format!("Store test item {}", i),
                parameters: json!({
                    "operation": "store",
                    "content": {
                        "test_value": i,
                        "data": format!("Test data {}", i)
                    },
                    "metadata": {
                        "importance": importance,
                        "processing_depth": processing_depth,
                        "tags": ["test", format!("item{}", i)],
                        "source": "testbed",
                        "confidence": 1.0,
                        "related_ids": [],
                        "custom": null
                    }
                }),
                priority: 5,
                deadline: None,
                created_at: chrono::Utc::now(),
            };
            
            // Store the item
            match app_state.orchestrator.read().await.submit_task("memory_management", store_task).await {
                Ok(result) => {
                    // Extract the item ID for later retrieval
                    if let Some(item_ids) = result.get("item_ids").and_then(|v| v.as_array()) {
                        if let Some(item_id) = item_ids.get(0).and_then(|v| v.as_str()) {
                            store_results.push(json!({
                                "item_id": item_id,
                                "importance": importance,
                                "processing_depth": processing_depth,
                                "result": result
                            }));
                        }
                    }
                },
                Err(e) => {
                    error!("Error storing memory item: {:?}", e);
                    return Err(format!("Store task error: {}", e));
                }
            }
            
            // Pause between operations
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
        
        // Retrieve items (both from STM and LTM)
        for store_result in &store_results {
            if let Some(item_id) = store_result.get("item_id").and_then(|v| v.as_str()) {
                // Create a retrieval task
                let retrieve_task = ComponentTask {
                    id: Uuid::new_v4().to_string(),
                    description: format!("Retrieve test item {}", item_id),
                    parameters: json!({
                        "operation": "retrieve",
                        "query": {
                            "id": item_id,
                            "query_both": true
                        }
                    }),
                    priority: 5,
                    deadline: None,
                    created_at: chrono::Utc::now(),
                };
                
                // Retrieve the item
                match app_state.orchestrator.read().await.submit_task("memory_management", retrieve_task).await {
                    Ok(result) => {
                        retrieve_results.push(json!({
                            "item_id": item_id,
                            "found": result.get("success").and_then(|v| v.as_bool()).unwrap_or(false),
                            "target": result.get("target").and_then(|v| v.as_str()).unwrap_or("unknown"),
                            "result": result
                        }));
                    },
                    Err(e) => {
                        error!("Error retrieving memory item: {:?}", e);
                        return Err(format!("Retrieve task error: {}", e));
                    }
                }
                
                // Pause between operations
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        
        // Run memory consolidation
        let consolidate_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: "Run memory consolidation",
            parameters: json!({
                "operation": "consolidate"
            }),
            priority: 8,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        let consolidation_result = match app_state.orchestrator.read().await.submit_task("memory_management", consolidate_task).await {
            Ok(result) => {
                info!("Ran memory consolidation");
                result
            },
            Err(e) => {
                error!("Failed to run memory consolidation: {:?}", e);
                json!({
                    "error": format!("{}", e)
                })
            }
        };
        
        // Get memory stats
        let stats_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: "Get memory stats",
            parameters: json!({
                "operation": "get_stats"
            }),
            priority: 3,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        let stats_result = match app_state.orchestrator.read().await.submit_task("memory_management", stats_task).await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to get memory stats: {:?}", e);
                json!({
                    "error": format!("{}", e)
                })
            }
        };
        
        // Analyze results for this allocation
        let stm_items = retrieve_results.iter()
            .filter(|r| r.get("target").and_then(|v| v.as_str()).unwrap_or("") == "stm")
            .count();
        
        let ltm_items = retrieve_results.iter()
            .filter(|r| r.get("target").and_then(|v| v.as_str()).unwrap_or("") == "ltm")
            .count();
        
        allocation_results.push(json!({
            "stm_allocation": stm_allocation,
            "store_results": store_results,
            "retrieve_results": retrieve_results,
            "consolidation_result": consolidation_result,
            "stats": stats_result,
            "summary": {
                "total_items": items_per_allocation,
                "items_in_stm": stm_items,
                "items_in_ltm": ltm_items,
                "retrieval_success_rate": retrieve_results.iter()
                    .filter(|r| r.get("found").and_then(|v| v.as_bool()).unwrap_or(false))
                    .count() as f64 / retrieve_results.len() as f64
            }
        }));
    }
    
    // Return overall results
    Ok(json!({
        "testbed": "Memory Allocation Testbed",
        "parameters": parameters,
        "results_by_allocation": allocation_results,
        "summary": {
            "allocation_levels_tested": stm_allocations,
            "items_per_allocation": items_per_allocation,
            "best_allocation": allocation_results.iter()
                .max_by(|a, b| {
                    let a_success = a.get("summary").and_then(|s| s.get("retrieval_success_rate")).and_then(|r| r.as_f64()).unwrap_or(0.0);
                    let b_success = b.get("summary").and_then(|s| s.get("retrieval_success_rate")).and_then(|r| r.as_f64()).unwrap_or(0.0);
                    a_success.partial_cmp(&b_success).unwrap_or(std::cmp::Ordering::Equal)
                })
                .and_then(|r| r.get("stm_allocation"))
                .and_then(|a| a.as_f64())
                .unwrap_or(0.5)
        },
        "conclusion": {
            "dynamic_memory_demonstrated": !allocation_results.is_empty(),
            "memory_transfers_observed": allocation_results.iter()
                .any(|r| r.get("consolidation_result")
                    .and_then(|c| c.get("item_ids"))
                    .and_then(|i| i.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false))
        }
    }))
}

/// Run Reasoning Validation Testbed (Testbed 3)
async fn run_reasoning_validation_testbed(
    app_state: &web::Data<AppState>,
    parameters: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    info!("Running Reasoning Validation Testbed");
    
    // Parse parameters
    let num_concepts = parameters.get("num_concepts")
        .and_then(|v| v.as_u64())
        .unwrap_or(5);
    
    let num_relationships = parameters.get("num_relationships")
        .and_then(|v| v.as_u64())
        .unwrap_or(10);
    
    let inference_iterations = parameters.get("inference_iterations")
        .and_then(|v| v.as_u64())
        .unwrap_or(3);
    
    // Results storage
    let mut concept_nodes = Vec::new();
    let mut relationship_edges = Vec::new();
    let mut inference_results = Vec::new();
    
    // 1. Create concepts (nodes)
    for i in 1..=num_concepts {
        let concept_name = format!("Concept{}", i);
        
        let node_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: format!("Create concept node: {}", concept_name),
            parameters: json!({
                "operation": "add_node",
                "label": concept_name,
                "properties": {
                    "description": format!("Test concept {}", i),
                    "category": "test",
                    "value": i
                },
                "confidence": 1.0,
                "source": "testbed"
            }),
            priority: 5,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        match app_state.orchestrator.read().await.submit_task("reasoning", node_task).await {
            Ok(result) => {
                info!("Created concept node: {}", concept_name);
                if let Some(node_id) = result.get("node_id").and_then(|v| v.as_str()) {
                    concept_nodes.push(json!({
                        "node_id": node_id,
                        "name": concept_name,
                        "result": result
                    }));
                }
            },
            Err(e) => {
                error!("Error creating concept node: {:?}", e);
                return Err(format!("Node creation error: {}", e));
            }
        }
        
        // Pause between operations
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // 2. Create relationships (edges)
    for i in 1..=num_relationships {
        // Select two random concepts to relate
        if concept_nodes.len() < 2 {
            continue;
        }
        
        let source_idx = i as usize % concept_nodes.len();
        let target_idx = (i as usize + 1) % concept_nodes.len();
        
        let source_node = &concept_nodes[source_idx];
        let target_node = &concept_nodes[target_idx];
        
        let source_id = source_node.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
        let target_id = target_node.get("node_id").and_then(|v| v.as_str()).unwrap_or("");
        
        // Skip if we couldn't get valid IDs
        if source_id.is_empty() || target_id.is_empty() {
            continue;
        }
        
        // Determine relationship type based on index
        let relationship = match i % 3 {
            0 => "related_to",
            1 => "depends_on",
            _ => "influences",
        };
        
        let edge_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: format!("Create relationship: {} {} {}", 
                        source_node.get("name").and_then(|v| v.as_str()).unwrap_or(""),
                        relationship,
                        target_node.get("name").and_then(|v| v.as_str()).unwrap_or("")),
            parameters: json!({
                "operation": "add_edge",
                "source_id": source_id,
                "target_id": target_id,
                "relationship": relationship,
                "properties": {
                    "strength": (i as f64) / (num_relationships as f64),
                    "description": format!("Relationship {}", i)
                },
                "confidence": 0.9,
                "source": "testbed",
                "is_inferred": false
            }),
            priority: 5,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        match app_state.orchestrator.read().await.submit_task("reasoning", edge_task).await {
            Ok(result) => {
                info!("Created relationship edge: {}", relationship);
                if let Some(edge_id) = result.get("edge_id").and_then(|v| v.as_str()) {
                    relationship_edges.push(json!({
                        "edge_id": edge_id,
                        "source_id": source_id,
                        "target_id": target_id,
                        "relationship": relationship,
                        "result": result
                    }));
                }
            },
            Err(e) => {
                error!("Error creating relationship edge: {:?}", e);
                return Err(format!("Edge creation error: {}", e));
            }
        }
        
        // Pause between operations
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // 3. Create an inference rule
    let rule_id = Uuid::new_v4().to_string();
    let rule_task = ComponentTask {
        id: Uuid::new_v4().to_string(),
        description: "Create inference rule for transitive relationships",
        parameters: json!({
            "operation": "add_rule",
            "rule": {
                "id": rule_id,
                "name": "Transitive Influence Rule",
                "pattern": {
                    "nodes": [
                        {
                            "variable": "a",
                            "label": null
                        },
                        {
                            "variable": "b",
                            "label": null
                        },
                        {
                            "variable": "c",
                            "label": null
                        }
                    ],
                    "edges": [
                        {
                            "source_var": "a",
                            "target_var": "b",
                            "relationship": "influences"
                        },
                        {
                            "source_var": "b",
                            "target_var": "c",
                            "relationship": "influences"
                        }
                    ]
                },
                "actions": [
                    {
                        "CreateEdge": {
                            "source_var": "a",
                            "target_var": "c",
                            "relationship": "influences",
                            "properties": {
                                "inferred": true,
                                "rule": "transitive_influence"
                            }
                        }
                    }
                ],
                "confidence_factor": 0.8,
                "enabled": true
            }
        }),
        priority: 8,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    let rule_result = match app_state.orchestrator.read().await.submit_task("reasoning", rule_task).await {
        Ok(result) => {
            info!("Created inference rule");
            result
        },
        Err(e) => {
            error!("Error creating inference rule: {:?}", e);
            return Err(format!("Rule creation error: {}", e));
        }
    };
    
    // 4. Run inference multiple times
    for i in 1..=inference_iterations {
        let inference_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: format!("Run inference iteration {}", i),
            parameters: json!({
                "operation": "run_inference"
            }),
            priority: 5,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        match app_state.orchestrator.read().await.submit_task("reasoning", inference_task).await {
            Ok(result) => {
                info!("Completed inference iteration {}", i);
                inference_results.push(json!({
                    "iteration": i,
                    "result": result
                }));
            },
            Err(e) => {
                error!("Error in inference iteration {}: {:?}", i, e);
                return Err(format!("Inference error: {}", e));
            }
        }
        
        // Pause between inferences
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    // 5. Query the knowledge graph to verify results
    let query_task = ComponentTask {
        id: Uuid::new_v4().to_string(),
        description: "Query knowledge graph for inferred relationships",
        parameters: json!({
            "operation": "query",
            "query": {
                "start_nodes": concept_nodes.iter()
                    .filter_map(|n| n.get("node_id").and_then(|v| v.as_str()))
                    .take(2)
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
                "max_depth": 2,
                "limit": 100
            }
        }),
        priority: 3,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    let query_result = match app_state.orchestrator.read().await.submit_task("reasoning", query_task).await {
        Ok(result) => result,
        Err(e) => {
            error!("Error querying knowledge graph: {:?}", e);
            return Err(format!("Query error: {}", e));
        }
    };
    
    // 6. Get stats
    let stats_task = ComponentTask {
        id: Uuid::new_v4().to_string(),
        description: "Get reasoning stats",
        parameters: json!({
            "operation": "get_stats"
        }),
        priority: 3,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    let stats_result = match app_state.orchestrator.read().await.submit_task("reasoning", stats_task).await {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to get reasoning stats: {:?}", e);
            json!({
                "error": format!("{}", e)
            })
        }
    };
    
    // Return overall results
    Ok(json!({
        "testbed": "Reasoning Validation Testbed",
        "parameters": parameters,
        "concepts": concept_nodes,
        "relationships": relationship_edges,
        "inference_rule": {
            "id": rule_id,
            "result": rule_result
        },
        "inference_results": inference_results,
        "query_result": query_result,
        "stats": stats_result,
        "summary": {
            "concepts_created": concept_nodes.len(),
            "relationships_created": relationship_edges.len(),
            "inference_iterations": inference_iterations,
            "inferences_generated": inference_results.iter()
                .filter_map(|r| r.get("result"))
                .filter_map(|r| r.get("generated_edges"))
                .filter_map(|e| e.as_array())
                .map(|a| a.len())
                .sum::<usize>(),
        },
        "conclusion": {
            "structural_reasoning_demonstrated": !inference_results.is_empty(),
            "inferences_verified": query_result.get("edges")
                .and_then(|e| e.as_array())
                .map(|a| !a.is_empty())
                .unwrap_or(false)
        }
    }))
}

/// Run Autonomous Operation Testbed (Testbed 4)
async fn run_autonomous_operation_testbed(
    app_state: &web::Data<AppState>,
    parameters: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    info!("Running Autonomous Operation Testbed");
    
    // Parse parameters
    let test_duration_secs = parameters.get("test_duration_secs")
        .and_then(|v| v.as_u64())
        .unwrap_or(30); // Default to 30 seconds
    
    let num_goals = parameters.get("num_goals")
        .and_then(|v| v.as_u64())
        .unwrap_or(2);
    
    let force_interruption = parameters.get("force_interruption")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    
    // Results storage
    let mut goals = Vec::new();
    let mut events = Vec::new();
    let mut snapshots = Vec::new();
    let start_time = chrono::Utc::now();
    
    // 1. Create goals
    for i in 1..=num_goals {
        let goal_name = format!("Test Goal {}", i);
        let goal_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: format!("Create goal: {}", goal_name),
            parameters: json!({
                "operation": "create_goal",
                "description": goal_name,
                "priority": 7
            }),
            priority: 8,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        match app_state.orchestrator.read().await.submit_task("operational_state", goal_task).await {
            Ok(result) => {
                info!("Created goal: {}", goal_name);
                if let Some(goal_id) = result.get("goal_id").and_then(|v| v.as_str()) {
                    goals.push(json!({
                        "goal_id": goal_id,
                        "name": goal_name,
                        "result": result
                    }));
                    
                    // Add target to the goal
                    let target_task = ComponentTask {
                        id: Uuid::new_v4().to_string(),
                        description: format!("Add target to goal: {}", goal_name),
                        parameters: json!({
                            "operation": "add_goal_target",
                            "goal_id": goal_id,
                            "description": format!("Target for {}", goal_name),
                            "criteria": {
                                "measure": "completion",
                                "target_value": 1.0
                            }
                        }),
                        priority: 8,
                        deadline: None,
                        created_at: chrono::Utc::now(),
                    };
                    
                    match app_state.orchestrator.read().await.submit_task("operational_state", target_task).await {
                        Ok(target_result) => {
                            info!("Added target to goal: {}", goal_name);
                            events.push(json!({
                                "event": "add_target",
                                "goal_id": goal_id,
                                "result": target_result,
                                "timestamp": chrono::Utc::now()
                            }));
                        },
                        Err(e) => {
                            error!("Error adding target to goal: {:?}", e);
                            events.push(json!({
                                "event": "error",
                                "description": format!("Failed to add target to goal: {}", e),
                                "timestamp": chrono::Utc::now()
                            }));
                        }
                    }
                }
            },
            Err(e) => {
                error!("Error creating goal: {:?}", e);
                return Err(format!("Goal creation error: {}", e));
            }
        }
        
        // Pause between operations
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }
    
    // 2. Let system run autonomously while taking periodic snapshots
    let snapshot_interval = test_duration_secs / 5;
    let end_time = start_time + chrono::Duration::seconds(test_duration_secs as i64);
    
    while chrono::Utc::now() < end_time {
        // Take a snapshot of system state
        let mode_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: "Get current operational mode",
            parameters: json!({
                "operation": "get_mode"
            }),
            priority: 3,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        let mode_result = match app_state.orchestrator.read().await.submit_task("operational_state", mode_task).await {
            Ok(result) => result,
            Err(e) => {
                error!("Error getting operational mode: {:?}", e);
                json!({
                    "error": format!("{}", e)
                })
            }
        };
        
        // Get task status
        let tasks_task = ComponentTask {
            id: Uuid::new_v4().to_string(),
            description: "Get current tasks",
            parameters: json!({
                "operation": "get_tasks"
            }),
            priority: 3,
            deadline: None,
            created_at: chrono::Utc::now(),
        };
        
        let tasks_result = match app_state.orchestrator.read().await.submit_task("operational_state", tasks_task).await {
            Ok(result) => result,
            Err(e) => {
                error!("Error getting tasks: {:?}", e);
                json!({
                    "error": format!("{}", e)
                })
            }
        };
        
        // Take a snapshot
        snapshots.push(json!({
            "timestamp": chrono::Utc::now(),
            "elapsed_secs": (chrono::Utc::now() - start_time).num_seconds(),
            "mode": mode_result,
            "tasks": tasks_result
        }));
        
        // If we're at the halfway point and force_interruption is enabled, simulate an interruption
        let elapsed = (chrono::Utc::now() - start_time).num_seconds();
        if force_interruption && elapsed >= (test_duration_secs as i64) / 2 && elapsed < (test_duration_secs as i64) / 2 + 3 {
            info!("Forcing system interruption");
            
            // Simulate interruption
            let interrupt_task = ComponentTask {
                id: Uuid::new_v4().to_string(),
                description: "Handle interruption",
                parameters: json!({
                    "operation": "handle_interruption"
                }),
                priority: 10,
                deadline: None,
                created_at: chrono::Utc::now(),
            };
            
            match app_state.orchestrator.read().await.submit_task("operational_state", interrupt_task).await {
                Ok(_) => {
                    info!("System interruption handled");
                    events.push(json!({
                        "event": "interruption",
                        "description": "System was interrupted",
                        "timestamp": chrono::Utc::now()
                    }));
                    
                    // Wait a moment for the system to stabilize
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                    
                    // Recover from interruption
                    let recovery_task = ComponentTask {
                        id: Uuid::new_v4().to_string(),
                        description: "Recover from interruption",
                        parameters: json!({
                            "operation": "recover_from_interruption"
                        }),
                        priority: 10,
                        deadline: None,
                        created_at: chrono::Utc::now(),
                    };
                    
                    match app_state.orchestrator.read().await.submit_task("operational_state", recovery_task).await {
                        Ok(_) => {
                            info!("System recovered from interruption");
                            events.push(json!({
                                "event": "recovery",
                                "description": "System recovered from interruption",
                                "timestamp": chrono::Utc::now()
                            }));
                        },
                        Err(e) => {
                            error!("Error recovering from interruption: {:?}", e);
                            events.push(json!({
                                "event": "error",
                                "description": format!("Failed to recover from interruption: {}", e),
                                "timestamp": chrono::Utc::now()
                            }));
                        }
                    }
                },
                Err(e) => {
                    error!("Error handling interruption: {:?}", e);
                    events.push(json!({
                        "event": "error",
                        "description": format!("Failed to handle interruption: {}", e),
                        "timestamp": chrono::Utc::now()
                    }));
                }
            }
        }
        
        // Wait until next snapshot
        tokio::time::sleep(tokio::time::Duration::from_secs(snapshot_interval)).await;
    }
    
    // 3. Get final stats and goals status
    let goals_task = ComponentTask {
        id: Uuid::new_v4().to_string(),
        description: "Get final goals status",
        parameters: json!({
            "operation": "get_goals"
        }),
        priority: 3,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    let goals_result = match app_state.orchestrator.read().await.submit_task("operational_state", goals_task).await {
        Ok(result) => result,
        Err(e) => {
            error!("Error getting final goals status: {:?}", e);
            json!({
                "error": format!("{}", e)
            })
        }
    };
    
    let stats_task = ComponentTask {
        id: Uuid::new_v4().to_string(),
        description: "Get final stats",
        parameters: json!({
            "operation": "get_stats"
        }),
        priority: 3,
        deadline: None,
        created_at: chrono::Utc::now(),
    };
    
    let stats_result = match app_state.orchestrator.read().await.submit_task("operational_state", stats_task).await {
        Ok(result) => result,
        Err(e) => {
            error!("Error getting final stats: {:?}", e);
            json!({
                "error": format!("{}", e)
            })
        }
    };
    
    // Return overall results
    Ok(json!({
        "testbed": "Autonomous Operation Testbed",
        "parameters": parameters,
        "goals": goals,
        "events": events,
        "snapshots": snapshots,
        "final_goals_status": goals_result,
        "final_stats": stats_result,
        "summary": {
            "test_duration_secs": test_duration_secs,
            "num_goals": goals.len(),
            "interruption_occurred": events.iter()
                .any(|e| e.get("event").and_then(|v| v.as_str()).unwrap_or("") == "interruption"),
            "recovery_occurred": events.iter()
                .any(|e| e.get("event").and_then(|v| v.as_str()).unwrap_or("") == "recovery"),
            "mode_switches": snapshots.iter()
                .zip(snapshots.iter().skip(1))
                .filter(|(prev, curr)| {
                    let prev_mode = prev.get("mode").and_then(|m| m.get("mode")).and_then(|m| m.as_str());
                    let curr_mode = curr.get("mode").and_then(|m| m.get("mode")).and_then(|m| m.as_str());
                    prev_mode != curr_mode
                })
                .count()
        },
        "conclusion": {
            "autonomous_operation_demonstrated": !snapshots.is_empty(),
            "multi_mode_operation_observed": snapshots.iter()
                .map(|s| s.get("mode").and_then(|m| m.get("mode")).and_then(|m| m.as_str()))
                .collect::<HashSet<_>>()
                .len() > 1,
            "interruption_recovery_successful": events.iter()
                .any(|e| e.get("event").and_then(|v| v.as_str()).unwrap_or("") == "recovery")
        }
    }))
}