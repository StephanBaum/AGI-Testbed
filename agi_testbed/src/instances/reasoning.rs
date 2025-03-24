use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use async_trait::async_trait;
use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use serde_json::json;
use chrono::Utc;

use crate::core::component::{
    Component, ComponentStatus, ComponentMessage, ComponentTask, 
    ComponentError, ComponentMetrics, ComponentConfig
};
use crate::instances::common::{
    BaseComponent, EventType, measure_execution_time, extract_task_param
};

/// Inference-Based Reasoning Core Instance
///
/// This component maintains structural knowledge representations and performs logical inference.
/// It provides capabilities for combining knowledge fragments for new insights and
/// validating logical consistency of inferences.
#[derive(Debug)]
pub struct ReasoningInstance {
    /// Base component functionality
    base: BaseComponent,
    /// Knowledge graph for storing relationships
    knowledge_graph: Arc<RwLock<KnowledgeGraph>>,
    /// Inference rules repository
    inference_rules: Arc<RwLock<Vec<InferenceRule>>>,
    /// Reasoning history for tracking inference paths
    reasoning_history: Arc<Mutex<Vec<ReasoningStep>>>,
    /// Current validation settings
    validation_settings: ValidationSettings,
    /// Stats tracking
    stats: Arc<RwLock<ReasoningStats>>,
}

/// Knowledge graph representation
#[derive(Debug, Clone, Default)]
pub struct KnowledgeGraph {
    /// Nodes in the graph (concepts)
    nodes: HashMap<String, KnowledgeNode>,
    /// Edges in the graph (relationships)
    edges: Vec<KnowledgeEdge>,
}

/// Node in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    /// Node identifier
    pub id: String,
    /// Node label/type
    pub label: String,
    /// Node properties
    pub properties: serde_json::Value,
    /// Node confidence (0-1)
    pub confidence: f64,
    /// Source of this knowledge
    pub source: String,
    /// Timestamp when created
    pub created_at: chrono::DateTime<Utc>,
}

/// Edge in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    /// Edge identifier
    pub id: String,
    /// Source node ID
    pub source_id: String,
    /// Target node ID
    pub target_id: String,
    /// Relationship type
    pub relationship: String,
    /// Edge properties
    pub properties: serde_json::Value,
    /// Edge confidence (0-1)
    pub confidence: f64,
    /// Source of this relationship
    pub source: String,
    /// Whether this edge was inferred or directly observed
    pub is_inferred: bool,
    /// Timestamp when created
    pub created_at: chrono::DateTime<Utc>,
}

/// Inference rule for generating new knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRule {
    /// Rule identifier
    pub id: String,
    /// Rule name/description
    pub name: String,
    /// Pattern to match in the knowledge graph
    pub pattern: RulePattern,
    /// Actions to take when pattern matches
    pub actions: Vec<RuleAction>,
    /// Confidence adjustment for inferences (0-1)
    pub confidence_factor: f64,
    /// Whether the rule is enabled
    pub enabled: bool,
}

/// Pattern to match in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulePattern {
    /// Node patterns to match
    pub nodes: Vec<NodePattern>,
    /// Edge patterns to match
    pub edges: Vec<EdgePattern>,
    /// Conditions that must be satisfied
    pub conditions: Option<serde_json::Value>,
}

/// Pattern for matching nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePattern {
    /// Variable name for binding
    pub variable: String,
    /// Node label to match (optional)
    pub label: Option<String>,
    /// Property constraints (optional)
    pub properties: Option<serde_json::Value>,
}

/// Pattern for matching edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgePattern {
    /// Source node variable
    pub source_var: String,
    /// Target node variable
    pub target_var: String,
    /// Relationship type to match (optional)
    pub relationship: Option<String>,
    /// Property constraints (optional)
    pub properties: Option<serde_json::Value>,
}

/// Action to take when a rule pattern matches
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleAction {
    /// Create a new node
    CreateNode {
        /// Node label
        label: String,
        /// Node properties (can reference variables)
        properties: serde_json::Value,
    },
    /// Create a new edge
    CreateEdge {
        /// Source node variable
        source_var: String,
        /// Target node variable
        target_var: String,
        /// Relationship type
        relationship: String,
        /// Edge properties (can reference variables)
        properties: serde_json::Value,
    },
    /// Update an existing node
    UpdateNode {
        /// Node variable to update
        variable: String,
        /// Properties to update
        properties: serde_json::Value,
    },
    /// Update an existing edge
    UpdateEdge {
        /// Source node variable
        source_var: String,
        /// Target node variable
        target_var: String,
        /// Properties to update
        properties: serde_json::Value,
    },
}

/// Reasoning step for tracking inference paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    /// Step identifier
    pub id: String,
    /// Rule applied
    pub rule_id: String,
    /// Matched nodes by variable
    pub matched_nodes: HashMap<String, String>,
    /// Matched edges
    pub matched_edges: Vec<String>,
    /// Generated nodes
    pub generated_nodes: Vec<String>,
    /// Generated edges
    pub generated_edges: Vec<String>,
    /// Confidence of this inference
    pub confidence: f64,
    /// Timestamp
    pub timestamp: chrono::DateTime<Utc>,
}

/// Settings for logical validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSettings {
    /// Check for logical contradictions
    pub check_contradictions: bool,
    /// Perform consistency checks on new inferences
    pub validate_new_inferences: bool,
    /// Confidence threshold for accepting inferences
    pub confidence_threshold: f64,
    /// Log all validation attempts
    pub log_validation: bool,
}

/// Reasoning operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    /// Operation success status
    pub success: bool,
    /// Operation description
    pub operation: String,
    /// Generated nodes
    pub generated_nodes: Vec<String>,
    /// Generated edges
    pub generated_edges: Vec<String>,
    /// Confidence of result
    pub confidence: f64,
    /// Reasoning steps that led to this result
    pub reasoning_steps: Vec<ReasoningStep>,
    /// Error message if any
    pub error: Option<String>,
}

/// Query for interrogating the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeQuery {
    /// Starting node(s) for the query
    pub start_nodes: Vec<String>,
    /// Pattern to traverse
    pub pattern: Option<RulePattern>,
    /// Plain text query (will be parsed)
    pub text_query: Option<String>,
    /// Minimum confidence level for results
    pub min_confidence: Option<f64>,
    /// Maximum path length to follow
    pub max_depth: Option<u32>,
    /// Maximum results to return
    pub limit: Option<usize>,
}

/// Statistics for tracking reasoning engine performance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ReasoningStats {
    /// Total number of inference operations
    pub total_inferences: u64,
    /// Number of successful inferences
    pub successful_inferences: u64,
    /// Number of rejected inferences (e.g., due to low confidence)
    pub rejected_inferences: u64,
    /// Number of contradictions detected
    pub contradictions_detected: u64,
    /// Average confidence of accepted inferences
    pub avg_inference_confidence: f64,
    /// Average processing time for inference operations (ms)
    pub avg_inference_time_ms: f64,
    /// Total nodes in knowledge graph
    pub total_nodes: usize,
    /// Total edges in knowledge graph
    pub total_edges: usize,
    /// Total rules in the system
    pub total_rules: usize,
}

impl ReasoningInstance {
    /// Create a new reasoning instance
    pub fn new() -> Self {
        Self {
            base: BaseComponent::new("reasoning", "ReasoningInstance"),
            knowledge_graph: Arc::new(RwLock::new(KnowledgeGraph::default())),
            inference_rules: Arc::new(RwLock::new(Vec::new())),
            reasoning_history: Arc::new(Mutex::new(Vec::new())),
            validation_settings: ValidationSettings {
                check_contradictions: true,
                validate_new_inferences: true,
                confidence_threshold: 0.6,
                log_validation: true,
            },
            stats: Arc::new(RwLock::new(ReasoningStats::default())),
        }
    }
    
    /// Add a node to the knowledge graph
    pub async fn add_node(
        &self,
        label: &str,
        properties: serde_json::Value,
        confidence: f64,
        source: &str,
    ) -> Result<String, ComponentError> {
        let id = uuid::Uuid::new_v4().to_string();
        
        let node = KnowledgeNode {
            id: id.clone(),
            label: label.to_string(),
            properties,
            confidence,
            source: source.to_string(),
            created_at: Utc::now(),
        };
        
        // Add to knowledge graph
        {
            let mut graph = self.knowledge_graph.write().await;
            graph.nodes.insert(id.clone(), node);
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.total_nodes = graph.nodes.len();
        }
        
        debug!("Added node: {} ({})", id, label);
        Ok(id)
    }
    
    /// Add an edge to the knowledge graph
    pub async fn add_edge(
        &self,
        source_id: &str,
        target_id: &str,
        relationship: &str,
        properties: serde_json::Value,
        confidence: f64,
        source: &str,
        is_inferred: bool,
    ) -> Result<String, ComponentError> {
        let id = uuid::Uuid::new_v4().to_string();
        
        // Verify nodes exist
        {
            let graph = self.knowledge_graph.read().await;
            if !graph.nodes.contains_key(source_id) {
                return Err(ComponentError::ValidationError(
                    format!("Source node {} does not exist", source_id)
                ));
            }
            
            if !graph.nodes.contains_key(target_id) {
                return Err(ComponentError::ValidationError(
                    format!("Target node {} does not exist", target_id)
                ));
            }
        }
        
        let edge = KnowledgeEdge {
            id: id.clone(),
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            relationship: relationship.to_string(),
            properties,
            confidence,
            source: source.to_string(),
            is_inferred,
            created_at: Utc::now(),
        };
        
        // Add to knowledge graph
        {
            let mut graph = self.knowledge_graph.write().await;
            graph.edges.push(edge);
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.total_edges = graph.edges.len();
        }
        
        debug!("Added edge: {} ({} -> {})", id, source_id, target_id);
        Ok(id)
    }
    
    /// Add an inference rule
    pub async fn add_rule(&self, rule: InferenceRule) -> Result<(), ComponentError> {
        // Validate rule
        if rule.pattern.nodes.is_empty() {
            return Err(ComponentError::ValidationError(
                "Rule pattern must contain at least one node".to_string()
            ));
        }
        
        // Add to rules repository
        {
            let mut rules = self.inference_rules.write().await;
            rules.push(rule.clone());
            
            // Update stats
            let mut stats = self.stats.write().await;
            stats.total_rules = rules.len();
        }
        
        debug!("Added inference rule: {} ({})", rule.id, rule.name);
        Ok(())
    }
    
    /// Run inference on the knowledge graph
    pub async fn run_inference(&self) -> Result<ReasoningResult, ComponentError> {
        debug!("Running inference process");
        
        let start_time = std::time::Instant::now();
        let mut generated_nodes = Vec::new();
        let mut generated_edges = Vec::new();
        let mut reasoning_steps = Vec::new();
        
        // Get all rules
        let rules = {
            let rules_guard = self.inference_rules.read().await;
            rules_guard.clone()
        };
        
        // For each enabled rule
        for rule in rules.iter().filter(|r| r.enabled) {
            debug!("Applying rule: {}", rule.name);
            
            // Find matches for the rule pattern
            let matches = self.find_pattern_matches(&rule.pattern).await?;
            
            for pattern_match in matches {
                // Apply rule actions to each match
                let step_result = self.apply_rule_actions(rule, &pattern_match).await?;
                
                if let Some(step) = step_result {
                    generated_nodes.extend(step.generated_nodes.clone());
                    generated_edges.extend(step.generated_edges.clone());
                    reasoning_steps.push(step);
                }
            }
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            let elapsed = start_time.elapsed().as_millis() as f64;
            
            stats.total_inferences += 1;
            if !reasoning_steps.is_empty() {
                stats.successful_inferences += 1;
                
                // Update average confidence
                let total_confidence: f64 = reasoning_steps.iter().map(|s| s.confidence).sum();
                let avg_confidence = total_confidence / reasoning_steps.len() as f64;
                
                stats.avg_inference_confidence = (
                    stats.avg_inference_confidence * (stats.successful_inferences - 1) as f64 + avg_confidence
                ) / stats.successful_inferences as f64;
            } else {
                stats.rejected_inferences += 1;
            }
            
            // Update average processing time
            stats.avg_inference_time_ms = (
                stats.avg_inference_time_ms * (stats.total_inferences - 1) as f64 + elapsed
            ) / stats.total_inferences as f64;
        }
        
        // Create result
        let avg_confidence = if !reasoning_steps.is_empty() {
            reasoning_steps.iter().map(|s| s.confidence).sum::<f64>() / reasoning_steps.len() as f64
        } else {
            0.0
        };
        
        Ok(ReasoningResult {
            success: !reasoning_steps.is_empty(),
            operation: "inference".to_string(),
            generated_nodes,
            generated_edges,
            confidence: avg_confidence,
            reasoning_steps,
            error: if reasoning_steps.is_empty() {
                Some("No new inferences could be made".to_string())
            } else {
                None
            },
        })
    }
    
    /// Query the knowledge graph
    pub async fn query_knowledge(
        &self,
        query: &KnowledgeQuery,
    ) -> Result<serde_json::Value, ComponentError> {
        debug!("Executing knowledge query");
        
        let graph = self.knowledge_graph.read().await;
        
        // Start with the requested nodes
        let mut result_nodes = HashSet::new();
        let mut result_edges = HashSet::new();
        
        for node_id in &query.start_nodes {
            if graph.nodes.contains_key(node_id) {
                result_nodes.insert(node_id.clone());
                
                // If pattern provided, follow it
                if let Some(pattern) = &query.pattern {
                    let matches = self.find_pattern_matches_from_node(node_id, pattern).await?;
                    
                    for pattern_match in matches {
                        // Add matched nodes and edges to results
                        for (_, node_id) in &pattern_match.matched_nodes {
                            result_nodes.insert(node_id.clone());
                        }
                        
                        for edge_id in &pattern_match.matched_edges {
                            result_edges.insert(edge_id.clone());
                        }
                    }
                } else {
                    // Without pattern, just return connected edges at depth 1
                    let depth = query.max_depth.unwrap_or(1);
                    self.traverse_graph(node_id, depth, &mut result_nodes, &mut result_edges).await?;
                }
            } else {
                warn!("Start node {} not found in knowledge graph", node_id);
            }
        }
        
        // Apply confidence filter
        let min_confidence = query.min_confidence.unwrap_or(0.0);
        
        let filtered_nodes: Vec<&KnowledgeNode> = result_nodes.iter()
            .filter_map(|id| graph.nodes.get(id))
            .filter(|node| node.confidence >= min_confidence)
            .collect();
        
        let filtered_edges: Vec<&KnowledgeEdge> = result_edges.iter()
            .filter_map(|id| graph.edges.iter().find(|e| e.id == *id))
            .filter(|edge| edge.confidence >= min_confidence)
            .collect();
        
        // Apply result limit
        let limit = query.limit.unwrap_or(100);
        let limited_nodes = if filtered_nodes.len() > limit {
            filtered_nodes[0..limit].to_vec()
        } else {
            filtered_nodes
        };
        
        // Format results
        Ok(json!({
            "nodes": limited_nodes,
            "edges": filtered_edges,
            "node_count": limited_nodes.len(),
            "edge_count": filtered_edges.len(),
            "more_results": filtered_nodes.len() > limit,
        }))
    }
    
    /// Check if a proposed addition would create a contradiction
    pub async fn check_contradiction(
        &self,
        proposed_edge: &KnowledgeEdge,
    ) -> Result<bool, ComponentError> {
        if !self.validation_settings.check_contradictions {
            return Ok(false);
        }
        
        let graph = self.knowledge_graph.read().await;
        
        // In a full implementation, this would check for various types of contradictions:
        // 1. Direct contradictions (e.g., A is-not B while also A is B)
        // 2. Transitive contradictions
        // 3. Property value contradictions
        // For the MVP, we'll implement a simple check for direct contradictions
        
        let contradicts = graph.edges.iter().any(|edge| 
            // Same source and target
            edge.source_id == proposed_edge.source_id && 
            edge.target_id == proposed_edge.target_id && 
            // Contradictory relationship
            (
                (edge.relationship == "is" && proposed_edge.relationship == "is-not") ||
                (edge.relationship == "is-not" && proposed_edge.relationship == "is") ||
                (edge.relationship == "greater-than" && proposed_edge.relationship == "less-than") ||
                (edge.relationship == "less-than" && proposed_edge.relationship == "greater-than") ||
                (edge.relationship == "equals" && proposed_edge.relationship == "not-equals") ||
                (edge.relationship == "not-equals" && proposed_edge.relationship == "equals")
            )
        );
        
        if contradicts {
            // Update stats
            let mut stats = self.stats.write().await;
            stats.contradictions_detected += 1;
            
            debug!("Contradiction detected for proposed edge: {}", proposed_edge.id);
        }
        
        Ok(contradicts)
    }
    
    /// Update validation settings
    pub fn update_validation_settings(&mut self, settings: ValidationSettings) {
        self.validation_settings = settings;
        debug!("Updated validation settings: {:?}", settings);
    }
    
    /// Get current reasoning statistics
    pub async fn get_stats(&self) -> ReasoningStats {
        self.stats.read().await.clone()
    }
    
    /// Get a specific node by ID
    pub async fn get_node(&self, id: &str) -> Option<KnowledgeNode> {
        let graph = self.knowledge_graph.read().await;
        graph.nodes.get(id).cloned()
    }
    
    /// Get a specific edge by ID
    pub async fn get_edge(&self, id: &str) -> Option<KnowledgeEdge> {
        let graph = self.knowledge_graph.read().await;
        graph.edges.iter().find(|e| e.id == id).cloned()
    }
    
    // Helper methods
    
    /// Find all matches for a rule pattern in the knowledge graph
    async fn find_pattern_matches(
        &self,
        pattern: &RulePattern,
    ) -> Result<Vec<PatternMatch>, ComponentError> {
        // This is a simplified pattern matching implementation
        // A full implementation would use a proper graph pattern matching algorithm
        
        let graph = self.knowledge_graph.read().await;
        let mut matches = Vec::new();
        
        // Start with each node that could match the first pattern node
        for (node_id, node) in &graph.nodes {
            let first_pattern = &pattern.nodes[0];
            
            // Check if this node matches the first pattern
            if self.node_matches_pattern(node, first_pattern) {
                let mut initial_match = PatternMatch {
                    matched_nodes: HashMap::new(),
                    matched_edges: Vec::new(),
                };
                
                initial_match.matched_nodes.insert(first_pattern.variable.clone(), node_id.clone());
                
                // Try to expand this match to cover the full pattern
                self.expand_pattern_match(&graph, pattern, initial_match, 1, &mut matches);
            }
        }
        
        Ok(matches)
    }
    
    /// Find pattern matches starting from a specific node
    async fn find_pattern_matches_from_node(
        &self,
        start_node_id: &str,
        pattern: &RulePattern,
    ) -> Result<Vec<PatternMatch>, ComponentError> {
        let graph = self.knowledge_graph.read().await;
        let mut matches = Vec::new();
        
        if let Some(node) = graph.nodes.get(start_node_id) {
            // Try matching the node with each pattern node
            for (i, pattern_node) in pattern.nodes.iter().enumerate() {
                if self.node_matches_pattern(node, pattern_node) {
                    let mut initial_match = PatternMatch {
                        matched_nodes: HashMap::new(),
                        matched_edges: Vec::new(),
                    };
                    
                    initial_match.matched_nodes.insert(pattern_node.variable.clone(), start_node_id.clone());
                    
                    // Expand from this initial match
                    // We need to be careful here to handle the case where we didn't start with the first pattern node
                    self.expand_pattern_match_from_middle(&graph, pattern, initial_match, i, &mut matches);
                }
            }
        }
        
        Ok(matches)
    }
    
    /// Check if a node matches a pattern node
    fn node_matches_pattern(
        &self,
        node: &KnowledgeNode,
        pattern: &NodePattern,
    ) -> bool {
        // Check label if specified
        if let Some(label) = &pattern.label {
            if node.label != *label {
                return false;
            }
        }
        
        // Check properties if specified
        if let Some(props) = &pattern.properties {
            // A real implementation would check property constraints
            // This is simplified for the MVP
        }
        
        true
    }
    
    /// Check if an edge matches a pattern edge
    fn edge_matches_pattern(
        &self,
        edge: &KnowledgeEdge,
        pattern: &EdgePattern,
        node_bindings: &HashMap<String, String>,
    ) -> bool {
        // Check source and target nodes
        let source_var = &pattern.source_var;
        let target_var = &pattern.target_var;
        
        if !node_bindings.contains_key(source_var) || !node_bindings.contains_key(target_var) {
            return false;
        }
        
        let source_id = &node_bindings[source_var];
        let target_id = &node_bindings[target_var];
        
        if edge.source_id != *source_id || edge.target_id != *target_id {
            return false;
        }
        
        // Check relationship if specified
        if let Some(rel) = &pattern.relationship {
            if edge.relationship != *rel {
                return false;
            }
        }
        
        // Check properties if specified
        if let Some(props) = &pattern.properties {
            // A real implementation would check property constraints
            // This is simplified for the MVP
        }
        
        true
    }
    
    /// Recursively expand a pattern match
    fn expand_pattern_match(
        &self,
        graph: &KnowledgeGraph,
        pattern: &RulePattern,
        current_match: PatternMatch,
        next_node_idx: usize,
        matches: &mut Vec<PatternMatch>,
    ) {
        // If we've matched all pattern nodes
        if next_node_idx >= pattern.nodes.len() {
            // Check if all required edges are matched
            let mut edge_match_complete = true;
            
            for edge_pattern in &pattern.edges {
                let source_var = &edge_pattern.source_var;
                let target_var = &edge_pattern.target_var;
                
                // Skip edges involving nodes we haven't matched yet
                if !current_match.matched_nodes.contains_key(source_var) || 
                   !current_match.matched_nodes.contains_key(target_var) {
                    edge_match_complete = false;
                    break;
                }
                
                let source_id = &current_match.matched_nodes[source_var];
                let target_id = &current_match.matched_nodes[target_var];
                
                let matching_edge = graph.edges.iter().find(|e| 
                    e.source_id == *source_id && 
                    e.target_id == *target_id && 
                    (edge_pattern.relationship.is_none() || 
                     e.relationship == edge_pattern.relationship.as_ref().unwrap())
                );
                
                if let Some(edge) = matching_edge {
                    // Make a new match with this edge included
                    let mut new_match = current_match.clone();
                    new_match.matched_edges.push(edge.id.clone());
                    
                    // Check if there are more edges to match
                    self.expand_edge_match(graph, pattern, new_match, 0, matches);
                } else {
                    edge_match_complete = false;
                    break;
                }
            }
            
            // If no edges to match or all edges matched
            if edge_match_complete && pattern.edges.is_empty() {
                matches.push(current_match);
            }
            
            return;
        }
        
        // Get the next pattern node
        let next_pattern = &pattern.nodes[next_node_idx];
        
        // Try each node in the graph
        for (node_id, node) in &graph.nodes {
            // Skip already matched nodes (no node can match multiple pattern parts)
            if current_match.matched_nodes.values().any(|id| id == node_id) {
                continue;
            }
            
            // Check if this node matches the pattern
            if self.node_matches_pattern(node, next_pattern) {
                // Create a new match with this node added
                let mut new_match = current_match.clone();
                new_match.matched_nodes.insert(next_pattern.variable.clone(), node_id.clone());
                
                // Continue expanding
                self.expand_pattern_match(graph, pattern, new_match, next_node_idx + 1, matches);
            }
        }
    }
    
    /// Expand pattern match when starting from a non-first node
    fn expand_pattern_match_from_middle(
        &self,
        graph: &KnowledgeGraph,
        pattern: &RulePattern,
        current_match: PatternMatch,
        start_idx: usize,
        matches: &mut Vec<PatternMatch>,
    ) {
        // This method is similar to expand_pattern_match but handles the case
        // where we're starting from a middle node in the pattern
        // For simplicity in the MVP, we'll just expand forward and backward separately
        
        // First expand forward to match later nodes
        let mut forward_matches = Vec::new();
        self.expand_pattern_match(graph, pattern, current_match.clone(), start_idx + 1, &mut forward_matches);
        
        // For each forward match, try to expand backward
        for forward_match in forward_matches {
            let mut backward_patterns = Vec::new();
            for i in 0..start_idx {
                backward_patterns.push(&pattern.nodes[i]);
            }
            
            // This is a simplified approach - a full implementation would be more sophisticated
            matches.push(forward_match);
        }
    }
    
    /// Expand edge matches after all nodes are matched
    fn expand_edge_match(
        &self,
        graph: &KnowledgeGraph,
        pattern: &RulePattern,
        current_match: PatternMatch,
        edge_idx: usize,
        matches: &mut Vec<PatternMatch>,
    ) {
        // If we've matched all edges
        if edge_idx >= pattern.edges.len() {
            matches.push(current_match);
            return;
        }
        
        // Get the next edge pattern
        let edge_pattern = &pattern.edges[edge_idx];
        let source_var = &edge_pattern.source_var;
        let target_var = &edge_pattern.target_var;
        
        // Skip if we haven't matched the source or target nodes yet
        if !current_match.matched_nodes.contains_key(source_var) || 
           !current_match.matched_nodes.contains_key(target_var) {
            self.expand_edge_match(graph, pattern, current_match, edge_idx + 1, matches);
            return;
        }
        
        let source_id = &current_match.matched_nodes[source_var];
        let target_id = &current_match.matched_nodes[target_var];
        
        // Find matching edges
        let matching_edges: Vec<&KnowledgeEdge> = graph.edges.iter()
            .filter(|e| 
                e.source_id == *source_id && 
                e.target_id == *target_id && 
                (edge_pattern.relationship.is_none() || 
                 e.relationship == edge_pattern.relationship.as_ref().unwrap())
            )
            .collect();
        
        if matching_edges.is_empty() {
            // No matching edge, skip this pattern
            return;
        }
        
        for edge in matching_edges {
            // Create a new match with this edge added
            let mut new_match = current_match.clone();
            new_match.matched_edges.push(edge.id.clone());
            
            // Continue expanding
            self.expand_edge_match(graph, pattern, new_match, edge_idx + 1, matches);
        }
    }
    
    /// Apply rule actions to a pattern match
    async fn apply_rule_actions(
        &self,
        rule: &InferenceRule,
        pattern_match: &PatternMatch,
    ) -> Result<Option<ReasoningStep>, ComponentError> {
        let step_id = uuid::Uuid::new_v4().to_string();
        let mut step = ReasoningStep {
            id: step_id,
            rule_id: rule.id.clone(),
            matched_nodes: pattern_match.matched_nodes.clone(),
            matched_edges: pattern_match.matched_edges.clone(),
            generated_nodes: Vec::new(),
            generated_edges: Vec::new(),
            confidence: rule.confidence_factor,
            timestamp: Utc::now(),
        };
        
        let graph = self.knowledge_graph.read().await;
        
        // Apply each action in the rule
        for action in &rule.actions {
            match action {
                RuleAction::CreateNode { label, properties } => {
                    let label = self.substitute_variables(label, pattern_match, &graph)?;
                    let props = self.substitute_variables_in_json(properties, pattern_match, &graph)?;
                    
                    // Create the new node
                    drop(graph); // Release the read lock
                    let node_id = self.add_node(
                        &label,
                        props,
                        rule.confidence_factor,
                        &format!("inference:{}", rule.id),
                    ).await?;
                    
                    step.generated_nodes.push(node_id);
                    let graph = self.knowledge_graph.read().await; // Re-acquire the read lock
                },
                RuleAction::CreateEdge { source_var, target_var, relationship, properties } => {
                    if !pattern_match.matched_nodes.contains_key(source_var) || 
                       !pattern_match.matched_nodes.contains_key(target_var) {
                        return Err(ComponentError::ValidationError(
                            format!("Referenced variables not in match: {} or {}", source_var, target_var)
                        ));
                    }
                    
                    let source_id = &pattern_match.matched_nodes[source_var];
                    let target_id = &pattern_match.matched_nodes[target_var];
                    let rel = self.substitute_variables(relationship, pattern_match, &graph)?;
                    let props = self.substitute_variables_in_json(properties, pattern_match, &graph)?;
                    
                    // Check for contradictions if enabled
                    let proposed_edge = KnowledgeEdge {
                        id: uuid::Uuid::new_v4().to_string(),
                        source_id: source_id.clone(),
                        target_id: target_id.clone(),
                        relationship: rel.clone(),
                        properties: props.clone(),
                        confidence: rule.confidence_factor,
                        source: format!("inference:{}", rule.id),
                        is_inferred: true,
                        created_at: Utc::now(),
                    };
                    
                    let contradicts = if self.validation_settings.validate_new_inferences {
                        drop(graph); // Release the read lock
                        let result = self.check_contradiction(&proposed_edge).await?;
                        let graph = self.knowledge_graph.read().await; // Re-acquire the read lock
                        result
                    } else {
                        false
                    };
                    
                    if contradicts {
                        debug!("Skipping contradictory edge creation");
                        continue;
                    }
                    
                    // Check confidence threshold
                    if rule.confidence_factor < self.validation_settings.confidence_threshold {
                        debug!("Skipping edge creation due to low confidence: {}", rule.confidence_factor);
                        continue;
                    }
                    
                    // Create the new edge
                    drop(graph); // Release the read lock
                    let edge_id = self.add_edge(
                        source_id,
                        target_id,
                        &rel,
                        props,
                        rule.confidence_factor,
                        &format!("inference:{}", rule.id),
                        true,
                    ).await?;
                    
                    step.generated_edges.push(edge_id);
                    let graph = self.knowledge_graph.read().await; // Re-acquire the read lock
                },
                RuleAction::UpdateNode { variable, properties } => {
                    // Not implemented in MVP
                    debug!("UpdateNode action not implemented in MVP");
                },
                RuleAction::UpdateEdge { source_var, target_var, properties } => {
                    // Not implemented in MVP
                    debug!("UpdateEdge action not implemented in MVP");
                },
            }
        }
        
        // Add step to reasoning history if any nodes or edges were generated
        if !step.generated_nodes.is_empty() || !step.generated_edges.is_empty() {
            let mut history = self.reasoning_history.lock().await;
            history.push(step.clone());
            
            // Keep history from growing too large
            if history.len() > 1000 {
                history.drain(0..500);
            }
            
            Ok(Some(step))
        } else {
            Ok(None)
        }
    }
    
    /// Substitute variables in a string using the pattern match
    fn substitute_variables(
        &self,
        template: &str,
        pattern_match: &PatternMatch,
        graph: &KnowledgeGraph,
    ) -> Result<String, ComponentError> {
        let mut result = template.to_string();
        
        // Simple variable substitution with format $VAR
        for (var, node_id) in &pattern_match.matched_nodes {
            let placeholder = format!("${}", var);
            
            if result.contains(&placeholder) {
                if let Some(node) = graph.nodes.get(node_id) {
                    // You could substitute with different node properties 
                    // For simplicity, we'll just use the node's label
                    result = result.replace(&placeholder, &node.label);
                }
            }
        }
        
        Ok(result)
    }
    
    /// Substitute variables in a JSON value using the pattern match
    fn substitute_variables_in_json(
        &self,
        template: &serde_json::Value,
        pattern_match: &PatternMatch,
        graph: &KnowledgeGraph,
    ) -> Result<serde_json::Value, ComponentError> {
        // Convert to string, substitute, then parse back
        // This is inefficient but simple for the MVP
        let template_str = template.to_string();
        let result_str = self.substitute_variables(&template_str, pattern_match, graph)?;
        
        // Parse back to JSON
        match serde_json::from_str(&result_str) {
            Ok(json) => Ok(json),
            Err(e) => Err(ComponentError::ValidationError(
                format!("Error parsing JSON after substitution: {}", e)
            )),
        }
    }
    
    /// Traverse the graph starting from a node to the specified depth
    async fn traverse_graph(
        &self,
        start_node_id: &str,
        depth: u32,
        result_nodes: &mut HashSet<String>,
        result_edges: &mut HashSet<String>,
    ) -> Result<(), ComponentError> {
        if depth == 0 {
            return Ok(());
        }
        
        let graph = self.knowledge_graph.read().await;
        
        // Find all edges connected to this node
        let connected_edges: Vec<&KnowledgeEdge> = graph.edges.iter()
            .filter(|e| e.source_id == start_node_id || e.target_id == start_node_id)
            .collect();
        
        for edge in connected_edges {
            result_edges.insert(edge.id.clone());
            
            let next_node_id = if edge.source_id == start_node_id {
                &edge.target_id
            } else {
                &edge.source_id
            };
            
            result_nodes.insert(next_node_id.clone());
            
            // Recursively traverse if not at max depth
            if depth > 1 {
                drop(graph); // Release the read lock
                self.traverse_graph(next_node_id, depth - 1, result_nodes, result_edges).await?;
                let graph = self.knowledge_graph.read().await; // Re-acquire the read lock
            }
        }
        
        Ok(())
    }
}

/// Pattern match result used internally
#[derive(Debug, Clone)]
struct PatternMatch {
    /// Matched nodes by variable name
    matched_nodes: HashMap<String, String>,
    /// Matched edge IDs
    matched_edges: Vec<String>,
}

#[async_trait]
impl Component for ReasoningInstance {
    fn id(&self) -> &str {
        &self.base.id
    }
    
    fn component_type(&self) -> &str {
        &self.base.component_type
    }
    
    fn status(&self) -> ComponentStatus {
        self.base.status.clone()
    }
    
    async fn initialize(&mut self, config: ComponentConfig) -> Result<(), ComponentError> {
        info!("Initializing ReasoningInstance with config: {}", config.id);
        self.base.config = Some(config.clone());
        
        // Extract configuration parameters
        if let Some(validation) = config.parameters.get("validation") {
            if let Some(check_contradictions) = validation.get("check_contradictions").and_then(|v| v.as_bool()) {
                self.validation_settings.check_contradictions = check_contradictions;
            }
            
            if let Some(validate_inferences) = validation.get("validate_new_inferences").and_then(|v| v.as_bool()) {
                self.validation_settings.validate_new_inferences = validate_inferences;
            }
            
            if let Some(confidence_threshold) = validation.get("confidence_threshold").and_then(|v| v.as_f64()) {
                self.validation_settings.confidence_threshold = confidence_threshold;
            }
            
            if let Some(log_validation) = validation.get("log_validation").and_then(|v| v.as_bool()) {
                self.validation_settings.log_validation = log_validation;
            }
        }
        
        // Load any predefined rules from config
        if let Some(rules) = config.parameters.get("rules").and_then(|v| v.as_array()) {
            for rule_json in rules {
                match serde_json::from_value::<InferenceRule>(rule_json.clone()) {
                    Ok(rule) => {
                        self.add_rule(rule).await?;
                    },
                    Err(e) => {
                        warn!("Failed to parse rule from config: {}", e);
                    }
                }
            }
        }
        
        self.base.status = ComponentStatus::Initialized;
        self.base.log_event(
            EventType::Initialization, 
            "ReasoningInstance initialized", 
            Some(json!({
                "validation_settings": self.validation_settings,
                "initial_rules": self.inference_rules.read().await.len(),
            }))
        ).await;
        
        Ok(())
    }
    
    async fn start(&mut self) -> Result<(), ComponentError> {
        info!("Starting ReasoningInstance");
        self.base.status = ComponentStatus::Running;
        
        self.base.log_event(
            EventType::StateChange, 
            "ReasoningInstance started", 
            None
        ).await;
        
        Ok(())
    }
    
    async fn pause(&mut self) -> Result<(), ComponentError> {
        info!("Pausing ReasoningInstance");
        self.base.status = ComponentStatus::Paused;
        
        self.base.log_event(
            EventType::StateChange, 
            "ReasoningInstance paused", 
            None
        ).await;
        
        Ok(())
    }
    
    async fn resume(&mut self) -> Result<(), ComponentError> {
        info!("Resuming ReasoningInstance");
        self.base.status = ComponentStatus::Running;
        
        self.base.log_event(
            EventType::StateChange, 
            "ReasoningInstance resumed", 
            None
        ).await;
        
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), ComponentError> {
        info!("Shutting down ReasoningInstance");
        self.base.status = ComponentStatus::ShuttingDown;
        
        self.base.log_event(
            EventType::StateChange, 
            "ReasoningInstance shutting down", 
            None
        ).await;
        
        Ok(())
    }
    
    async fn process_task(&mut self, task: ComponentTask) -> Result<serde_json::Value, ComponentError> {
        debug!("Processing task: {}", task.id);
        self.base.log_event(
            EventType::TaskProcessing, 
            &format!("Processing task {}", task.id), 
            Some(json!({"description": task.description}))
        ).await;
        
        // Determine the operation type from task parameters
        let operation = extract_task_param::<String>(&task, "operation")
            .unwrap_or_else(|_| "query".to_string());
        
        let (result, duration_ms) = match operation.as_str() {
            "add_node" => {
                let label = extract_task_param::<String>(&task, "label")?;
                let properties = extract_task_param::<serde_json::Value>(&task, "properties")
                    .unwrap_or_else(|_| json!({}));
                let confidence = extract_task_param::<f64>(&task, "confidence")
                    .unwrap_or(1.0);
                let source = extract_task_param::<String>(&task, "source")
                    .unwrap_or_else(|_| "user".to_string());
                
                let node_id_future = self.add_node(&label, properties, confidence, &source);
                let (node_id, duration) = measure_execution_time(node_id_future).await;
                
                (
                    node_id.map(|id| json!({
                        "success": true,
                        "operation": "add_node",
                        "node_id": id,
                    })),
                    duration
                )
            },
            "add_edge" => {
                let source_id = extract_task_param::<String>(&task, "source_id")?;
                let target_id = extract_task_param::<String>(&task, "target_id")?;
                let relationship = extract_task_param::<String>(&task, "relationship")?;
                let properties = extract_task_param::<serde_json::Value>(&task, "properties")
                    .unwrap_or_else(|_| json!({}));
                let confidence = extract_task_param::<f64>(&task, "confidence")
                    .unwrap_or(1.0);
                let source = extract_task_param::<String>(&task, "source")
                    .unwrap_or_else(|_| "user".to_string());
                let is_inferred = extract_task_param::<bool>(&task, "is_inferred")
                    .unwrap_or(false);
                
                let edge_id_future = self.add_edge(
                    &source_id, &target_id, &relationship, 
                    properties, confidence, &source, is_inferred
                );
                let (edge_id, duration) = measure_execution_time(edge_id_future).await;
                
                (
                    edge_id.map(|id| json!({
                        "success": true,
                        "operation": "add_edge",
                        "edge_id": id,
                    })),
                    duration
                )
            },
            "add_rule" => {
                let rule = extract_task_param::<InferenceRule>(&task, "rule")?;
                
                let result_future = self.add_rule(rule.clone());
                let (result, duration) = measure_execution_time(result_future).await;
                
                (
                    result.map(|_| json!({
                        "success": true,
                        "operation": "add_rule",
                        "rule_id": rule.id,
                    })),
                    duration
                )
            },
            "run_inference" => {
                let result_future = self.run_inference();
                measure_execution_time(result_future).await
            },
            "query" => {
                let query = extract_task_param::<KnowledgeQuery>(&task, "query")?;
                
                let result_future = self.query_knowledge(&query);
                measure_execution_time(result_future).await
            },
            "get_stats" => {
                let stats_future = self.get_stats();
                
                let (stats, duration) = measure_execution_time(stats_future).await;
                (Ok(serde_json::to_value(stats).unwrap()), duration)
            },
            "update_validation" => {
                let settings = extract_task_param::<ValidationSettings>(&task, "settings")?;
                
                self.update_validation_settings(settings.clone());
                
                (
                    Ok(json!({
                        "success": true,
                        "operation": "update_validation",
                        "settings": settings,
                    })),
                    0.0
                )
            },
            _ => {
                return Err(ComponentError::ValidationError(format!(
                    "Unknown operation: {}", operation
                )));
            }
        };
        
        // Record task processing in metrics
        self.base.record_task_processing(result.is_ok(), duration_ms);
        
        // Return result
        match result {
            Ok(value) => Ok(value),
            Err(e) => Err(e),
        }
    }
    
    async fn handle_message(&mut self, message: ComponentMessage) -> Result<(), ComponentError> {
        debug!("Handling message: {}", message.id);
        self.base.log_event(
            EventType::Info, 
            &format!("Received message {}", message.id), 
            Some(json!({"sender": message.sender, "type": message.message_type}))
        ).await;
        
        // Handle different message types
        match message.message_type.as_str() {
            "add_knowledge" => {
                if let Some(content) = message.payload.get("content") {
                    // A more sophisticated implementation would parse and add knowledge
                    // from natural language or structured content
                    debug!("Add knowledge message received but not fully implemented");
                }
                Ok(())
            },
            "run_inference" => {
                self.run_inference().await?;
                Ok(())
            },
            _ => {
                // Unknown message type
                warn!("Received unknown message type: {}", message.message_type);
                Ok(())
            }
        }
    }
    
    async fn collect_metrics(&self) -> Result<ComponentMetrics, ComponentError> {
        // Gather current metrics
        let mut metrics = self.base.get_metrics();
        
        // Get reasoning statistics
        let stats = self.stats.read().await;
        let knowledge_graph = self.knowledge_graph.read().await;
        
        // Add reasoning-specific metrics
        let custom_metrics = json!({
            "total_nodes": knowledge_graph.nodes.len(),
            "total_edges": knowledge_graph.edges.len(),
            "total_rules": self.inference_rules.read().await.len(),
            "total_inferences": stats.total_inferences,
            "successful_inferences": stats.successful_inferences,
            "rejected_inferences": stats.rejected_inferences,
            "contradictions_detected": stats.contradictions_detected,
            "avg_inference_confidence": stats.avg_inference_confidence,
            "avg_inference_time_ms": stats.avg_inference_time_ms,
        });
        
        metrics.custom_metrics = custom_metrics;
        
        Ok(metrics)
    }
    
    async fn export_state(&self) -> Result<serde_json::Value, ComponentError> {
        // Export component state for persistence
        let knowledge_graph = self.knowledge_graph.read().await;
        let rules = self.inference_rules.read().await;
        let stats = self.stats.read().await;
        
        let state = json!({
            "validation_settings": self.validation_settings,
            "stats": *stats,
            "rules": *rules,
            "knowledge_graph": {
                "node_count": knowledge_graph.nodes.len(),
                "edge_count": knowledge_graph.edges.len(),
                // Note: In a real system, graph would be exported to a separate file
            },
            "base": {
                "id": self.base.id,
                "component_type": self.base.component_type,
                "status": format!("{:?}", self.base.status),
                "metrics": {
                    "tasks_processed": self.base.metrics.tasks_processed,
                    "tasks_failed": self.base.metrics.tasks_failed,
                    "avg_processing_time": self.base.metrics.avg_processing_time,
                    "custom_metrics": self.base.metrics.custom_metrics,
                },
            }
        });
        
        Ok(state)
    }
    
    async fn import_state(&mut self, state: serde_json::Value) -> Result<(), ComponentError> {
        // Import previously exported state
        if let Some(validation) = state.get("validation_settings") {
            if let Ok(settings) = serde_json::from_value::<ValidationSettings>(validation.clone()) {
                self.validation_settings = settings;
            }
        }
        
        // Rules would be imported here in a full implementation
        
        Ok(())
    }
    
    fn get_info(&self) -> serde_json::Value {
        json!({
            "id": self.base.id,
            "type": self.base.component_type,
            "status": format!("{:?}", self.base.status),
            "validation": {
                "check_contradictions": self.validation_settings.check_contradictions,
                "validate_inferences": self.validation_settings.validate_new_inferences,
                "confidence_threshold": self.validation_settings.confidence_threshold,
            },
            "tasks_processed": self.base.metrics.tasks_processed,
        })
    }
    
    fn as_any(&self) -> &dyn Any {
        self
    }
    
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

impl Default for ReasoningInstance {
    fn default() -> Self {
        Self::new()
    }
}

// Tests for ReasoningInstance
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test creating a new reasoning instance
    #[test]
    fn test_new_instance() {
        let instance = ReasoningInstance::new();
        assert_eq!(instance.base.id, "reasoning");
        assert!(instance.validation_settings.confidence_threshold > 0.0);
    }
    
    // Additional tests would be implemented here
}