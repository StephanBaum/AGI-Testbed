use std::any::Any;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use async_trait::async_trait;
use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use serde_json::json;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use lru::LruCache;

use crate::core::component::{
    Component, ComponentStatus, ComponentMessage, ComponentTask, 
    ComponentError, ComponentMetrics, ComponentConfig
};
use crate::instances::common::{
    BaseComponent, EventType, measure_execution_time, extract_task_param
};

/// Continuous Memory Management Instance
///
/// This component dynamically shifts capacity between short-term and long-term memory storage.
/// It handles memory allocation, retrieval, and consolidation based on importance, processing
/// depth, and access frequency.
#[derive(Debug)]
pub struct MemoryManagementInstance {
    /// Base component functionality
    base: BaseComponent,
    /// Short-Term Memory store
    stm: Arc<Mutex<STM>>,
    /// Long-Term Memory store
    ltm: Arc<Mutex<LTM>>,
    /// Memory transfer parameters
    alpha: f64, // importance weight
    beta: f64,  // processing depth weight
    gamma: f64, // access frequency weight
    /// Threshold for memory transfer
    transfer_threshold: f64,
    /// Maximum STM capacity
    max_stm_capacity: usize,
    /// Maximum LTM capacity
    max_ltm_capacity: usize,
    /// Current STM allocation (percentage of total resources)
    stm_allocation: f64,
    /// Memory consolidation settings
    consolidation_settings: ConsolidationSettings,
}

/// Short-Term Memory implementation
#[derive(Debug)]
pub struct STM {
    /// Memory items stored in STM
    items: LruCache<String, MemoryItem>,
    /// Access frequency counter for each item
    access_counts: HashMap<String, u64>,
    /// Maximum capacity
    capacity: usize,
    /// Current usage statistics
    stats: MemoryStats,
}

/// Long-Term Memory implementation
#[derive(Debug)]
pub struct LTM {
    /// Memory items stored in LTM
    items: DashMap<String, MemoryItem>,
    /// Access frequency counter for each item
    access_counts: DashMap<String, u64>,
    /// Index by tags for faster retrieval
    tag_index: DashMap<String, Vec<String>>,
    /// Maximum capacity
    capacity: usize,
    /// Current usage statistics
    stats: MemoryStats,
}

/// Memory item stored in STM or LTM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique identifier
    pub id: String,
    /// Item content
    pub content: serde_json::Value,
    /// Item metadata
    pub metadata: MemoryItemMetadata,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,
    /// Last modified timestamp
    pub last_modified: DateTime<Utc>,
}

/// Metadata for memory items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItemMetadata {
    /// Item importance (0.0-1.0)
    pub importance: f64,
    /// Processing depth applied to this item (0.0-1.0)
    pub processing_depth: f64,
    /// Tags for categorization and retrieval
    pub tags: Vec<String>,
    /// Source of the memory (e.g., "perception", "reasoning", "external")
    pub source: String,
    /// Confidence in the memory (0.0-1.0)
    pub confidence: f64,
    /// Related memory IDs
    pub related_ids: Vec<String>,
    /// Custom metadata
    pub custom: Option<serde_json::Value>,
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Current item count
    pub item_count: usize,
    /// Memory utilization percentage
    pub utilization: f64,
    /// Average importance of stored items
    pub avg_importance: f64,
    /// Average access frequency
    pub avg_access_frequency: f64,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Settings for memory consolidation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationSettings {
    /// Enable automatic consolidation
    pub enabled: bool,
    /// Consolidation interval in seconds
    pub interval_secs: u64,
    /// Maximum items to consolidate per cycle
    pub max_items_per_cycle: usize,
    /// Importance threshold for consolidation (0.0-1.0)
    pub importance_threshold: f64,
}

/// Memory operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryOperationResult {
    /// Operation success status
    pub success: bool,
    /// Operation type
    pub operation: String,
    /// Target memory (STM or LTM)
    pub target: String,
    /// Affected item IDs
    pub item_ids: Vec<String>,
    /// Additional result data
    pub data: Option<serde_json::Value>,
    /// Error message if any
    pub error: Option<String>,
}

/// Memory query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Query by specific ID
    pub id: Option<String>,
    /// Query by tags (logical OR between tags)
    pub tags: Option<Vec<String>>,
    /// Full-text search in content
    pub content_query: Option<String>,
    /// Minimum importance threshold
    pub min_importance: Option<f64>,
    /// Query both STM and LTM
    pub query_both: bool,
    /// Maximum results to return
    pub limit: Option<usize>,
}

impl MemoryManagementInstance {
    /// Create a new memory management instance
    pub fn new() -> Self {
        // Initialize with default values
        let max_stm_capacity = 1000;
        let max_ltm_capacity = 100000;
        
        Self {
            base: BaseComponent::new("memory_management", "MemoryManagementInstance"),
            stm: Arc::new(Mutex::new(STM {
                items: LruCache::new(max_stm_capacity),
                access_counts: HashMap::new(),
                capacity: max_stm_capacity,
                stats: MemoryStats {
                    item_count: 0,
                    utilization: 0.0,
                    avg_importance: 0.0,
                    avg_access_frequency: 0.0,
                    last_updated: Utc::now(),
                },
            })),
            ltm: Arc::new(Mutex::new(LTM {
                items: DashMap::new(),
                access_counts: DashMap::new(),
                tag_index: DashMap::new(),
                capacity: max_ltm_capacity,
                stats: MemoryStats {
                    item_count: 0,
                    utilization: 0.0,
                    avg_importance: 0.0,
                    avg_access_frequency: 0.0,
                    last_updated: Utc::now(),
                },
            })),
            alpha: 0.4, // Weight for importance
            beta: 0.3,  // Weight for processing depth
            gamma: 0.3, // Weight for access frequency
            transfer_threshold: 0.7,
            max_stm_capacity,
            max_ltm_capacity,
            stm_allocation: 0.2, // 20% STM, 80% LTM initial allocation
            consolidation_settings: ConsolidationSettings {
                enabled: true,
                interval_secs: 60,
                max_items_per_cycle: 100,
                importance_threshold: 0.3,
            },
        }
    }
    
    /// Store an item in memory
    pub async fn store(&self, 
        content: serde_json::Value, 
        metadata: MemoryItemMetadata,
        target: Option<&str>
    ) -> Result<MemoryOperationResult, ComponentError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now();
        
        let item = MemoryItem {
            id: id.clone(),
            content,
            metadata: metadata.clone(),
            created_at: now,
            last_accessed: now,
            last_modified: now,
        };
        
        // Calculate memory score to determine placement
        let memory_score = self.calculate_memory_score(
            metadata.importance, 
            metadata.processing_depth, 
            0.0 // Initial frequency is 0
        );
        
        let target = match target {
            Some("stm") => "stm",
            Some("ltm") => "ltm",
            _ => if memory_score >= self.transfer_threshold {
                "ltm" // High-value items go directly to LTM
            } else {
                "stm" // Default to STM for new items
            }
        };
        
        match target {
            "stm" => {
                let mut stm = self.stm.lock().await;
                // Add to STM
                stm.items.put(id.clone(), item);
                stm.access_counts.insert(id.clone(), 1);
                self.update_stm_stats(&mut stm).await;
                
                debug!("Stored item {} in STM", id);
                Ok(MemoryOperationResult {
                    success: true,
                    operation: "store".to_string(),
                    target: "stm".to_string(),
                    item_ids: vec![id],
                    data: None,
                    error: None,
                })
            },
            "ltm" => {
                let ltm = self.ltm.lock().await;
                // Add to LTM
                ltm.items.insert(id.clone(), item.clone());
                ltm.access_counts.insert(id.clone(), 1);
                
                // Update tag index
                for tag in &metadata.tags {
                    let mut ids = ltm.tag_index.entry(tag.clone()).or_insert_with(Vec::new);
                    ids.push(id.clone());
                }
                
                debug!("Stored item {} in LTM", id);
                // Asynchronously update stats
                let ltm_clone = self.ltm.clone();
                tokio::spawn(async move {
                    let ltm = ltm_clone.lock().await;
                    Self::update_ltm_stats(&ltm);
                });
                
                Ok(MemoryOperationResult {
                    success: true,
                    operation: "store".to_string(),
                    target: "ltm".to_string(),
                    item_ids: vec![id],
                    data: None,
                    error: None,
                })
            },
            _ => {
                Err(ComponentError::ValidationError(
                    format!("Invalid memory target: {}", target)
                ))
            }
        }
    }
    
    /// Retrieve an item from memory
    pub async fn retrieve(&self, 
        query: MemoryQuery
    ) -> Result<MemoryOperationResult, ComponentError> {
        // Check if looking for a specific ID
        if let Some(id) = &query.id {
            return self.retrieve_by_id(id, query.query_both).await;
        }
        
        // Perform tag-based or content-based query
        let mut results = Vec::new();
        let limit = query.limit.unwrap_or(10);
        
        // Search in STM if not explicitly looking in just LTM
        if query.query_both || query.tags.is_none() {
            let mut stm = self.stm.lock().await;
            
            // Filter items based on query criteria
            for (id, item) in stm.items.iter() {
                if self.matches_query(&item, &query) {
                    results.push(item.clone());
                    
                    // Update access count for this item
                    let count = stm.access_counts.entry(id.clone()).or_insert(0);
                    *count += 1;
                    
                    // Update last access time
                    if let Some(item) = stm.items.get_mut(id) {
                        item.last_accessed = Utc::now();
                    }
                    
                    if results.len() >= limit {
                        break;
                    }
                }
            }
            
            // Update STM stats after retrieval
            self.update_stm_stats(&mut stm).await;
        }
        
        // If still need more results or explicitly querying LTM
        if (query.query_both && results.len() < limit) || query.tags.is_some() {
            let ltm = self.ltm.lock().await;
            
            // If querying by tags, use the tag index for efficiency
            let candidate_ids = if let Some(tags) = &query.tags {
                let mut ids = Vec::new();
                for tag in tags {
                    if let Some(tag_ids) = ltm.tag_index.get(tag) {
                        ids.extend(tag_ids.value().clone());
                    }
                }
                // Deduplicate ids
                ids.sort();
                ids.dedup();
                ids
            } else {
                // Without tags, we need to scan all items (less efficient)
                ltm.items.iter().map(|entry| entry.key().clone()).collect()
            };
            
            // Process candidate IDs
            for id in candidate_ids {
                if let Some(item) = ltm.items.get(&id) {
                    let item_val = item.value();
                    
                    if self.matches_query(item_val, &query) {
                        results.push(item_val.clone());
                        
                        // Update access count
                        let mut count = ltm.access_counts.entry(id.clone()).or_insert(0);
                        *count += 1;
                        
                        // Update last access time
                        if let Some(mut item) = ltm.items.get_mut(&id) {
                            item.last_accessed = Utc::now();
                        }
                        
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
            
            // Update LTM stats asynchronously
            let ltm_clone = self.ltm.clone();
            tokio::spawn(async move {
                let ltm = ltm_clone.lock().await;
                Self::update_ltm_stats(&ltm);
            });
        }
        
        if results.is_empty() {
            return Ok(MemoryOperationResult {
                success: false,
                operation: "retrieve".to_string(),
                target: if query.query_both { "both".to_string() } else { "stm".to_string() },
                item_ids: Vec::new(),
                data: None,
                error: Some("No matching items found".to_string()),
            });
        }
        
        Ok(MemoryOperationResult {
            success: true,
            operation: "retrieve".to_string(),
            target: if query.query_both { "both".to_string() } else { "stm".to_string() },
            item_ids: results.iter().map(|item| item.id.clone()).collect(),
            data: Some(json!({
                "items": results,
                "count": results.len(),
            })),
            error: None,
        })
    }
    
    /// Retrieve an item by ID
    async fn retrieve_by_id(&self, id: &str, query_both: bool) -> Result<MemoryOperationResult, ComponentError> {
        // Try STM first
        let mut found_in_stm = false;
        let mut item = None;
        
        {
            let mut stm = self.stm.lock().await;
            if let Some(stm_item) = stm.items.get(id) {
                item = Some(stm_item.clone());
                found_in_stm = true;
                
                // Update access count and time
                let count = stm.access_counts.entry(id.to_string()).or_insert(0);
                *count += 1;
                
                if let Some(item) = stm.items.get_mut(id) {
                    item.last_accessed = Utc::now();
                }
                
                self.update_stm_stats(&mut stm).await;
            }
        }
        
        // Try LTM if not found in STM and allowed to query both
        if !found_in_stm && query_both {
            let ltm = self.ltm.lock().await;
            if let Some(ltm_item) = ltm.items.get(id) {
                item = Some(ltm_item.value().clone());
                
                // Update access count and time
                let mut count = ltm.access_counts.entry(id.to_string()).or_insert(0);
                *count += 1;
                
                if let Some(mut ltm_item) = ltm.items.get_mut(id) {
                    ltm_item.last_accessed = Utc::now();
                }
                
                // Update LTM stats asynchronously
                let ltm_clone = self.ltm.clone();
                tokio::spawn(async move {
                    let ltm = ltm_clone.lock().await;
                    Self::update_ltm_stats(&ltm);
                });
            }
        }
        
        if let Some(found_item) = item {
            Ok(MemoryOperationResult {
                success: true,
                operation: "retrieve".to_string(),
                target: if found_in_stm { "stm".to_string() } else { "ltm".to_string() },
                item_ids: vec![id.to_string()],
                data: Some(json!({
                    "items": [found_item],
                    "count": 1,
                })),
                error: None,
            })
        } else {
            Ok(MemoryOperationResult {
                success: false,
                operation: "retrieve".to_string(),
                target: if query_both { "both".to_string() } else { "stm".to_string() },
                item_ids: Vec::new(),
                data: None,
                error: Some(format!("Item with ID {} not found", id)),
            })
        }
    }
    
    /// Update an existing memory item
    pub async fn update(&self, 
        id: &str, 
        content: Option<serde_json::Value>,
        metadata: Option<MemoryItemMetadata>
    ) -> Result<MemoryOperationResult, ComponentError> {
        let now = Utc::now();
        let mut found = false;
        let mut target = "unknown";
        
        // Check STM first
        {
            let mut stm = self.stm.lock().await;
            if let Some(mut item) = stm.items.get_mut(id) {
                if let Some(content) = &content {
                    item.content = content.clone();
                }
                
                if let Some(metadata) = &metadata {
                    item.metadata = metadata.clone();
                }
                
                item.last_modified = now;
                item.last_accessed = now;
                
                // Update access counts
                let count = stm.access_counts.entry(id.to_string()).or_insert(0);
                *count += 1;
                
                found = true;
                target = "stm";
                self.update_stm_stats(&mut stm).await;
                
                // Check if item should be transferred to LTM
                let memory_score = self.calculate_memory_score(
                    item.metadata.importance, 
                    item.metadata.processing_depth, 
                    *count as f64
                );
                
                if memory_score >= self.transfer_threshold {
                    // Transfer to LTM
                    let item_clone = item.clone();
                    let stm_clone = self.stm.clone();
                    let ltm_clone = self.ltm.clone();
                    let id_clone = id.to_string();
                    
                    tokio::spawn(async move {
                        Self::transfer_to_ltm(
                            &stm_clone, 
                            &ltm_clone, 
                            &id_clone, 
                            item_clone
                        ).await;
                    });
                }
            }
        }
        
        // Check LTM if not found in STM
        if !found {
            let ltm = self.ltm.lock().await;
            if let Some(mut item) = ltm.items.get_mut(id) {
                if let Some(content) = &content {
                    item.content = content.clone();
                }
                
                if let Some(metadata) = &metadata {
                    // Update tag index if tags changed
                    if metadata.tags != item.metadata.tags {
                        // Remove old tags
                        for tag in &item.metadata.tags {
                            if let Some(mut tag_ids) = ltm.tag_index.get_mut(tag) {
                                tag_ids.retain(|t| t != id);
                            }
                        }
                        
                        // Add new tags
                        for tag in &metadata.tags {
                            let mut tag_ids = ltm.tag_index.entry(tag.clone()).or_insert_with(Vec::new);
                            if !tag_ids.contains(&id.to_string()) {
                                tag_ids.push(id.to_string());
                            }
                        }
                    }
                    
                    item.metadata = metadata.clone();
                }
                
                item.last_modified = now;
                item.last_accessed = now;
                
                // Update access counts
                let mut count = ltm.access_counts.entry(id.to_string()).or_insert(0);
                *count += 1;
                
                found = true;
                target = "ltm";
                
                // Update LTM stats asynchronously
                let ltm_clone = self.ltm.clone();
                tokio::spawn(async move {
                    let ltm = ltm_clone.lock().await;
                    Self::update_ltm_stats(&ltm);
                });
            }
        }
        
        if found {
            Ok(MemoryOperationResult {
                success: true,
                operation: "update".to_string(),
                target: target.to_string(),
                item_ids: vec![id.to_string()],
                data: None,
                error: None,
            })
        } else {
            Ok(MemoryOperationResult {
                success: false,
                operation: "update".to_string(),
                target: "both".to_string(),
                item_ids: Vec::new(),
                data: None,
                error: Some(format!("Item with ID {} not found", id)),
            })
        }
    }
    
    /// Delete a memory item
    pub async fn delete(&self, id: &str) -> Result<MemoryOperationResult, ComponentError> {
        let mut found = false;
        let mut target = "unknown";
        
        // Check STM first
        {
            let mut stm = self.stm.lock().await;
            if stm.items.contains(id) {
                stm.items.pop(id);
                stm.access_counts.remove(id);
                
                found = true;
                target = "stm";
                self.update_stm_stats(&mut stm).await;
            }
        }
        
        // Check LTM if not found in STM
        if !found {
            let ltm = self.ltm.lock().await;
            if let Some(item) = ltm.items.get(id) {
                // Remove from tag index
                for tag in &item.metadata.tags {
                    if let Some(mut tag_ids) = ltm.tag_index.get_mut(tag) {
                        tag_ids.retain(|t| t != id);
                    }
                }
                
                ltm.items.remove(id);
                ltm.access_counts.remove(id);
                
                found = true;
                target = "ltm";
                
                // Update LTM stats asynchronously
                let ltm_clone = self.ltm.clone();
                tokio::spawn(async move {
                    let ltm = ltm_clone.lock().await;
                    Self::update_ltm_stats(&ltm);
                });
            }
        }
        
        if found {
            Ok(MemoryOperationResult {
                success: true,
                operation: "delete".to_string(),
                target: target.to_string(),
                item_ids: vec![id.to_string()],
                data: None,
                error: None,
            })
        } else {
            Ok(MemoryOperationResult {
                success: false,
                operation: "delete".to_string(),
                target: "both".to_string(),
                item_ids: Vec::new(),
                data: None,
                error: Some(format!("Item with ID {} not found", id)),
            })
        }
    }
    
    /// Run memory consolidation process
    pub async fn consolidate_memory(&self) -> Result<MemoryOperationResult, ComponentError> {
        if !self.consolidation_settings.enabled {
            return Ok(MemoryOperationResult {
                success: false,
                operation: "consolidate".to_string(),
                target: "both".to_string(),
                item_ids: Vec::new(),
                data: None,
                error: Some("Consolidation is disabled".to_string()),
            });
        }
        
        // Get candidates from STM for transfer to LTM
        let transfer_candidates = {
            let stm = self.stm.lock().await;
            let mut candidates = Vec::new();
            
            for (id, item) in stm.items.iter() {
                let access_count = stm.access_counts.get(id).copied().unwrap_or(0);
                
                let memory_score = self.calculate_memory_score(
                    item.metadata.importance, 
                    item.metadata.processing_depth, 
                    access_count as f64
                );
                
                if memory_score >= self.transfer_threshold {
                    candidates.push((id.clone(), memory_score));
                }
            }
            
            // Sort by memory score (highest first)
            candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            
            // Limit to max items per cycle
            if candidates.len() > self.consolidation_settings.max_items_per_cycle {
                candidates.truncate(self.consolidation_settings.max_items_per_cycle);
            }
            
            candidates
        };
        
        // Transfer items from STM to LTM
        let mut transferred_ids = Vec::new();
        for (id, _) in transfer_candidates {
            let item_opt = {
                let mut stm = self.stm.lock().await;
                stm.items.pop(&id).map(|i| (i, stm.access_counts.remove(&id).unwrap_or(1)))
            };
            
            if let Some((item, _)) = item_opt {
                // Add to LTM
                let ltm = self.ltm.lock().await;
                ltm.items.insert(id.clone(), item.clone());
                ltm.access_counts.insert(id.clone(), 1); // Reset access count in LTM
                
                // Update tag index
                for tag in &item.metadata.tags {
                    let mut tag_ids = ltm.tag_index.entry(tag.clone()).or_insert_with(Vec::new);
                    tag_ids.push(id.clone());
                }
                
                transferred_ids.push(id);
                
                debug!("Transferred item {} from STM to LTM during consolidation", item.id);
            }
        }
        
        // Update stats asynchronously
        let stm_clone = self.stm.clone();
        let ltm_clone = self.ltm.clone();
        tokio::spawn(async move {
            let mut stm = stm_clone.lock().await;
            let self_static = MemoryManagementInstance::new();
            self_static.update_stm_stats(&mut stm).await;
            
            let ltm = ltm_clone.lock().await;
            Self::update_ltm_stats(&ltm);
        });
        
        Ok(MemoryOperationResult {
            success: true,
            operation: "consolidate".to_string(),
            target: "stm_to_ltm".to_string(),
            item_ids: transferred_ids.clone(),
            data: Some(json!({
                "transferred_count": transferred_ids.len(),
            })),
            error: None,
        })
    }
    
    /// Update memory allocation between STM and LTM
    pub async fn update_memory_allocation(&mut self, stm_allocation: f64) -> Result<(), ComponentError> {
        if stm_allocation < 0.1 || stm_allocation > 0.9 {
            return Err(ComponentError::ValidationError(
                "STM allocation must be between 0.1 and 0.9".to_string()
            ));
        }
        
        self.stm_allocation = stm_allocation;
        
        // Calculate new capacities based on allocation
        let total_capacity = self.max_stm_capacity + self.max_ltm_capacity;
        let new_stm_capacity = (total_capacity as f64 * stm_allocation) as usize;
        let new_ltm_capacity = total_capacity - new_stm_capacity;
        
        // Update STM capacity
        {
            let mut stm = self.stm.lock().await;
            stm.capacity = new_stm_capacity;
            // Note: LruCache would need to be recreated with new capacity
            // For simplicity in this MVP, we'll just update the capacity value
        }
        
        // Update LTM capacity
        {
            let ltm = self.ltm.lock().await;
            ltm.capacity = new_ltm_capacity;
        }
        
        info!("Updated memory allocation: STM={}%, LTM={}%", 
              stm_allocation * 100.0, (1.0 - stm_allocation) * 100.0);
        
        Ok(())
    }
    
    /// Update memory transfer parameters
    pub fn update_transfer_parameters(
        &mut self, 
        alpha: Option<f64>, 
        beta: Option<f64>, 
        gamma: Option<f64>,
        threshold: Option<f64>,
    ) -> Result<(), ComponentError> {
        if let Some(a) = alpha {
            if a < 0.0 || a > 1.0 {
                return Err(ComponentError::ValidationError("Alpha must be between 0 and 1".to_string()));
            }
            self.alpha = a;
        }
        
        if let Some(b) = beta {
            if b < 0.0 || b > 1.0 {
                return Err(ComponentError::ValidationError("Beta must be between 0 and 1".to_string()));
            }
            self.beta = b;
        }
        
        if let Some(g) = gamma {
            if g < 0.0 || g > 1.0 {
                return Err(ComponentError::ValidationError("Gamma must be between 0 and 1".to_string()));
            }
            self.gamma = g;
        }
        
        if let Some(t) = threshold {
            if t < 0.0 || t > 1.0 {
                return Err(ComponentError::ValidationError("Threshold must be between 0 and 1".to_string()));
            }
            self.transfer_threshold = t;
        }
        
        // Ensure weights sum to approximately 1.0
        let sum = self.alpha + self.beta + self.gamma;
        if sum < 0.99 || sum > 1.01 {
            // Normalize weights
            let factor = 1.0 / sum;
            self.alpha *= factor;
            self.beta *= factor;
            self.gamma *= factor;
            
            debug!("Normalized memory transfer weights: α={}, β={}, γ={}", 
                  self.alpha, self.beta, self.gamma);
        }
        
        Ok(())
    }
    
    /// Get memory statistics
    pub async fn get_memory_stats(&self) -> Result<serde_json::Value, ComponentError> {
        let stm_stats = {
            let stm = self.stm.lock().await;
            stm.stats.clone()
        };
        
        let ltm_stats = {
            let ltm = self.ltm.lock().await;
            ltm.stats.clone()
        };
        
        Ok(json!({
            "stm": {
                "capacity": self.max_stm_capacity,
                "item_count": stm_stats.item_count,
                "utilization": stm_stats.utilization,
                "avg_importance": stm_stats.avg_importance,
                "avg_access_frequency": stm_stats.avg_access_frequency,
            },
            "ltm": {
                "capacity": self.max_ltm_capacity,
                "item_count": ltm_stats.item_count,
                "utilization": ltm_stats.utilization,
                "avg_importance": ltm_stats.avg_importance,
                "avg_access_frequency": ltm_stats.avg_access_frequency,
            },
            "allocation": {
                "stm_percentage": self.stm_allocation * 100.0,
                "ltm_percentage": (1.0 - self.stm_allocation) * 100.0,
            },
            "transfer_parameters": {
                "alpha": self.alpha,
                "beta": self.beta,
                "gamma": self.gamma,
                "threshold": self.transfer_threshold,
            },
            "consolidation": {
                "enabled": self.consolidation_settings.enabled,
                "interval_secs": self.consolidation_settings.interval_secs,
                "importance_threshold": self.consolidation_settings.importance_threshold,
            }
        }))
    }
    
    /// Helper: Calculate memory score using the formula M(i, p, f) = α·i + β·p + γ·f
    fn calculate_memory_score(&self, importance: f64, processing_depth: f64, frequency: f64) -> f64 {
        let normalized_frequency = if frequency > 0.0 {
            (frequency.min(100.0) / 100.0).min(1.0)
        } else {
            0.0
        };
        
        self.alpha * importance + self.beta * processing_depth + self.gamma * normalized_frequency
    }
    
    /// Helper: Check if an item matches a query
    fn matches_query(&self, item: &MemoryItem, query: &MemoryQuery) -> bool {
        // Check importance threshold
        if let Some(min_importance) = query.min_importance {
            if item.metadata.importance < min_importance {
                return false;
            }
        }
        
        // Check tags (if any tag matches, include the item - OR logic)
        if let Some(query_tags) = &query.tags {
            if !query_tags.is_empty() {
                let mut matches_any_tag = false;
                for tag in query_tags {
                    if item.metadata.tags.contains(tag) {
                        matches_any_tag = true;
                        break;
                    }
                }
                
                if !matches_any_tag {
                    return false;
                }
            }
        }
        
        // Check content query (simple string matching for MVP)
        if let Some(content_query) = &query.content_query {
            if !content_query.is_empty() {
                let content_str = item.content.to_string().to_lowercase();
                if !content_str.contains(&content_query.to_lowercase()) {
                    return false;
                }
            }
        }
        
        true
    }
    
    /// Helper: Transfer an item from STM to LTM
    async fn transfer_to_ltm(
        stm: &Arc<Mutex<STM>>, 
        ltm: &Arc<Mutex<LTM>>, 
        id: &str, 
        item: MemoryItem
    ) {
        // Remove from STM
        {
            let mut stm_guard = stm.lock().await;
            stm_guard.items.pop(id);
            stm_guard.access_counts.remove(id);
        }
        
        // Add to LTM
        {
            let ltm_guard = ltm.lock().await;
            ltm_guard.items.insert(id.to_string(), item.clone());
            ltm_guard.access_counts.insert(id.to_string(), 1); // Reset access count
            
            // Update tag index
            for tag in &item.metadata.tags {
                let mut tag_ids = ltm_guard.tag_index.entry(tag.clone()).or_insert_with(Vec::new);
                tag_ids.push(id.to_string());
            }
        }
        
        debug!("Transferred item {} from STM to LTM", id);
    }
    
    /// Helper: Update STM stats
    async fn update_stm_stats(&self, stm: &mut STM) {
        let total_importance = stm.items.iter()
            .map(|(_, item)| item.metadata.importance)
            .sum::<f64>();
        
        let total_frequency = stm.access_counts.values().sum::<u64>() as f64;
        
        let item_count = stm.items.len();
        stm.stats = MemoryStats {
            item_count,
            utilization: if stm.capacity > 0 {
                item_count as f64 / stm.capacity as f64
            } else {
                0.0
            },
            avg_importance: if item_count > 0 {
                total_importance / item_count as f64
            } else {
                0.0
            },
            avg_access_frequency: if item_count > 0 {
                total_frequency / item_count as f64
            } else {
                0.0
            },
            last_updated: Utc::now(),
        };
    }
    
    /// Helper: Update LTM stats
    fn update_ltm_stats(ltm: &LTM) {
        let item_count = ltm.items.len();
        
        let total_importance: f64 = ltm.items.iter()
            .map(|entry| entry.value().metadata.importance)
            .sum();
        
        let total_frequency: f64 = ltm.access_counts.iter()
            .map(|entry| *entry.value() as f64)
            .sum();
        
        let stats = MemoryStats {
            item_count,
            utilization: if ltm.capacity > 0 {
                item_count as f64 / ltm.capacity as f64
            } else {
                0.0
            },
            avg_importance: if item_count > 0 {
                total_importance / item_count as f64
            } else {
                0.0
            },
            avg_access_frequency: if item_count > 0 {
                total_frequency / item_count as f64
            } else {
                0.0
            },
            last_updated: Utc::now(),
        };
        
        // Since ltm.stats is not atomic, we can't update it directly here
        // This is a simplified approach for the MVP
    }
    
    /// Start the memory consolidation background task
    pub async fn start_consolidation_task(&self) -> Result<(), ComponentError> {
        if !self.consolidation_settings.enabled {
            return Ok(());
        }
        
        let interval_secs = self.consolidation_settings.interval_secs;
        let instance = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval_secs)
            );
            
            loop {
                interval.tick().await;
                debug!("Running scheduled memory consolidation");
                
                if let Err(e) = instance.consolidate_memory().await {
                    error!("Error during memory consolidation: {:?}", e);
                }
            }
        });
        
        info!("Started memory consolidation task with interval {} seconds", interval_secs);
        Ok(())
    }
}

impl Clone for MemoryManagementInstance {
    fn clone(&self) -> Self {
        Self {
            base: BaseComponent::new(&self.base.id, &self.base.component_type),
            stm: self.stm.clone(),
            ltm: self.ltm.clone(),
            alpha: self.alpha,
            beta: self.beta,
            gamma: self.gamma,
            transfer_threshold: self.transfer_threshold,
            max_stm_capacity: self.max_stm_capacity,
            max_ltm_capacity: self.max_ltm_capacity,
            stm_allocation: self.stm_allocation,
            consolidation_settings: self.consolidation_settings.clone(),
        }
    }
}

#[async_trait]
impl Component for MemoryManagementInstance {
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
        info!("Initializing MemoryManagementInstance with config: {}", config.id);
        self.base.config = Some(config.clone());
        
        // Extract configuration parameters
        if let Some(stm_capacity) = config.parameters.get("max_stm_capacity").and_then(|v| v.as_u64()) {
            self.max_stm_capacity = stm_capacity as usize;
        }
        
        if let Some(ltm_capacity) = config.parameters.get("max_ltm_capacity").and_then(|v| v.as_u64()) {
            self.max_ltm_capacity = ltm_capacity as usize;
        }
        
        if let Some(alpha) = config.parameters.get("alpha").and_then(|v| v.as_f64()) {
            self.alpha = alpha;
        }
        
        if let Some(beta) = config.parameters.get("beta").and_then(|v| v.as_f64()) {
            self.beta = beta;
        }
        
        if let Some(gamma) = config.parameters.get("gamma").and_then(|v| v.as_f64()) {
            self.gamma = gamma;
        }
        
        if let Some(threshold) = config.parameters.get("transfer_threshold").and_then(|v| v.as_f64()) {
            self.transfer_threshold = threshold;
        }
        
        if let Some(stm_allocation) = config.parameters.get("stm_allocation").and_then(|v| v.as_f64()) {
            self.stm_allocation = stm_allocation;
        }
        
        // Initialize consolidation settings
        if let Some(consolidation) = config.parameters.get("consolidation") {
            if let Some(enabled) = consolidation.get("enabled").and_then(|v| v.as_bool()) {
                self.consolidation_settings.enabled = enabled;
            }
            
            if let Some(interval) = consolidation.get("interval_secs").and_then(|v| v.as_u64()) {
                self.consolidation_settings.interval_secs = interval;
            }
            
            if let Some(max_items) = consolidation.get("max_items_per_cycle").and_then(|v| v.as_u64()) {
                self.consolidation_settings.max_items_per_cycle = max_items as usize;
            }
            
            if let Some(threshold) = consolidation.get("importance_threshold").and_then(|v| v.as_f64()) {
                self.consolidation_settings.importance_threshold = threshold;
            }
        }
        
        // Ensure weights sum to 1.0
        let sum = self.alpha + self.beta + self.gamma;
        if (sum - 1.0).abs() > 0.01 {
            // Normalize weights
            let factor = 1.0 / sum;
            self.alpha *= factor;
            self.beta *= factor;
            self.gamma *= factor;
        }
        
        self.base.status = ComponentStatus::Initialized;
        self.base.log_event(
            EventType::Initialization, 
            "MemoryManagementInstance initialized", 
            Some(json!({
                "stm_capacity": self.max_stm_capacity,
                "ltm_capacity": self.max_ltm_capacity,
                "transfer_params": {
                    "alpha": self.alpha,
                    "beta": self.beta,
                    "gamma": self.gamma,
                    "threshold": self.transfer_threshold
                }
            }))
        ).await;
        
        Ok(())
    }
    
    async fn start(&mut self) -> Result<(), ComponentError> {
        info!("Starting MemoryManagementInstance");
        self.base.status = ComponentStatus::Running;
        
        // Start the memory consolidation task if enabled
        if self.consolidation_settings.enabled {
            self.start_consolidation_task().await?;
        }
        
        self.base.log_event(
            EventType::StateChange, 
            "MemoryManagementInstance started", 
            Some(json!({
                "consolidation_enabled": self.consolidation_settings.enabled
            }))
        ).await;
        
        Ok(())
    }
    
    async fn pause(&mut self) -> Result<(), ComponentError> {
        info!("Pausing MemoryManagementInstance");
        self.base.status = ComponentStatus::Paused;
        
        // In a full implementation, we would pause the consolidation task here
        
        self.base.log_event(
            EventType::StateChange, 
            "MemoryManagementInstance paused", 
            None
        ).await;
        
        Ok(())
    }
    
    async fn resume(&mut self) -> Result<(), ComponentError> {
        info!("Resuming MemoryManagementInstance");
        self.base.status = ComponentStatus::Running;
        
        // In a full implementation, we would resume the consolidation task here
        
        self.base.log_event(
            EventType::StateChange, 
            "MemoryManagementInstance resumed", 
            None
        ).await;
        
        Ok(())
    }
    
    async fn shutdown(&mut self) -> Result<(), ComponentError> {
        info!("Shutting down MemoryManagementInstance");
        self.base.status = ComponentStatus::ShuttingDown;
        
        // In a full implementation, we would stop the consolidation task here
        
        self.base.log_event(
            EventType::StateChange, 
            "MemoryManagementInstance shutting down", 
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
            .unwrap_or_else(|_| "retrieve".to_string());
        
        let (result, duration_ms) = match operation.as_str() {
            "store" => {
                let content = extract_task_param::<serde_json::Value>(&task, "content")?;
                let metadata = extract_task_param::<MemoryItemMetadata>(&task, "metadata")?;
                let target = task.parameters.get("target").and_then(|v| v.as_str());
                
                measure_execution_time(self.store(content, metadata, target)).await
            },
            "retrieve" => {
                let query = extract_task_param::<MemoryQuery>(&task, "query")?;
                measure_execution_time(self.retrieve(query)).await
            },
            "update" => {
                let id = extract_task_param::<String>(&task, "id")?;
                let content = task.parameters.get("content").cloned();
                let metadata = if let Some(meta_val) = task.parameters.get("metadata") {
                    Some(serde_json::from_value(meta_val.clone())?)
                } else {
                    None
                };
                
                measure_execution_time(self.update(&id, content, metadata)).await
            },
            "delete" => {
                let id = extract_task_param::<String>(&task, "id")?;
                measure_execution_time(self.delete(&id)).await
            },
            "consolidate" => {
                measure_execution_time(self.consolidate_memory()).await
            },
            "get_stats" => {
                measure_execution_time(self.get_memory_stats()).await
            },
            "update_allocation" => {
                let stm_allocation = extract_task_param::<f64>(&task, "stm_allocation")?;
                let result = self.update_memory_allocation(stm_allocation).await;
                (result.map(|_| json!({"success": true})), 0.0)
            },
            "update_parameters" => {
                let alpha = task.parameters.get("alpha").and_then(|v| v.as_f64());
                let beta = task.parameters.get("beta").and_then(|v| v.as_f64());
                let gamma = task.parameters.get("gamma").and_then(|v| v.as_f64());
                let threshold = task.parameters.get("threshold").and_then(|v| v.as_f64());
                
                let result = self.update_transfer_parameters(alpha, beta, gamma, threshold);
                (result.map(|_| json!({"success": true})), 0.0)
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
            "store_memory" => {
                if let (Some(content), Some(metadata)) = (
                    message.payload.get("content"),
                    message.payload.get("metadata")
                ) {
                    let metadata: MemoryItemMetadata = serde_json::from_value(metadata.clone())?;
                    let target = message.payload.get("target").and_then(|v| v.as_str());
                    
                    self.store(content.clone(), metadata, target).await?;
                }
                Ok(())
            },
            "consolidate_now" => {
                self.consolidate_memory().await?;
                Ok(())
            },
            "update_parameters" => {
                let alpha = message.payload.get("alpha").and_then(|v| v.as_f64());
                let beta = message.payload.get("beta").and_then(|v| v.as_f64());
                let gamma = message.payload.get("gamma").and_then(|v| v.as_f64());
                let threshold = message.payload.get("threshold").and_then(|v| v.as_f64());
                
                self.update_transfer_parameters(alpha, beta, gamma, threshold)
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
        
        // Get memory statistics
        let stm_stats = {
            let stm = self.stm.lock().await;
            stm.stats.clone()
        };
        
        let ltm_stats = {
            let ltm = self.ltm.lock().await;
            ltm.stats.clone()
        };
        
        // Add memory-management specific metrics
        let custom_metrics = json!({
            "stm_item_count": stm_stats.item_count,
            "stm_utilization": stm_stats.utilization,
            "ltm_item_count": ltm_stats.item_count,
            "ltm_utilization": ltm_stats.utilization,
            "stm_allocation": self.stm_allocation,
            "alpha": self.alpha,
            "beta": self.beta,
            "gamma": self.gamma,
            "transfer_threshold": self.transfer_threshold,
        });
        
        metrics.custom_metrics = custom_metrics;
        
        Ok(metrics)
    }
    
    async fn export_state(&self) -> Result<serde_json::Value, ComponentError> {
        // Export component state for persistence
        let state = json!({
            "alpha": self.alpha,
            "beta": self.beta,
            "gamma": self.gamma,
            "transfer_threshold": self.transfer_threshold,
            "stm_allocation": self.stm_allocation,
            "max_stm_capacity": self.max_stm_capacity,
            "max_ltm_capacity": self.max_ltm_capacity,
            "consolidation_settings": self.consolidation_settings,
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
            // Note: We don't include the actual memory contents here as that could be large
            // In a real implementation, this would reference a separate storage mechanism
        });
        
        Ok(state)
    }
    
    async fn import_state(&mut self, state: serde_json::Value) -> Result<(), ComponentError> {
        // Import previously exported state
        if let Some(alpha) = state.get("alpha").and_then(|v| v.as_f64()) {
            self.alpha = alpha;
        }
        
        if let Some(beta) = state.get("beta").and_then(|v| v.as_f64()) {
            self.beta = beta;
        }
        
        if let Some(gamma) = state.get("gamma").and_then(|v| v.as_f64()) {
            self.gamma = gamma;
        }
        
        if let Some(threshold) = state.get("transfer_threshold").and_then(|v| v.as_f64()) {
            self.transfer_threshold = threshold;
        }
        
        if let Some(allocation) = state.get("stm_allocation").and_then(|v| v.as_f64()) {
            self.stm_allocation = allocation;
        }
        
        if let Some(settings) = state.get("consolidation_settings") {
            if let Ok(config) = serde_json::from_value::<ConsolidationSettings>(settings.clone()) {
                self.consolidation_settings = config;
            }
        }
        
        // Other state would be restored as needed
        
        Ok(())
    }
    
    fn get_info(&self) -> serde_json::Value {
        json!({
            "id": self.base.id,
            "type": self.base.component_type,
            "status": format!("{:?}", self.base.status),
            "memory_params": {
                "alpha": self.alpha,
                "beta": self.beta,
                "gamma": self.gamma,
                "threshold": self.transfer_threshold
            },
            "allocation": {
                "stm": self.stm_allocation,
                "ltm": 1.0 - self.stm_allocation
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

impl Default for MemoryManagementInstance {
    fn default() -> Self {
        Self::new()
    }
}

// Tests for MemoryManagementInstance
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test creating a new memory management instance
    #[test]
    fn test_new_instance() {
        let instance = MemoryManagementInstance::new();
        assert_eq!(instance.base.id, "memory_management");
        assert!(instance.alpha > 0.0 && instance.alpha < 1.0);
        assert!(instance.beta > 0.0 && instance.beta < 1.0);
        assert!(instance.gamma > 0.0 && instance.gamma < 1.0);
        assert!((instance.alpha + instance.beta + instance.gamma - 1.0).abs() < 0.01);
    }
    
    // Test calculating memory score
    #[test]
    fn test_memory_score() {
        let instance = MemoryManagementInstance::new();
        
        let score1 = instance.calculate_memory_score(0.5, 0.5, 0.0);
        let score2 = instance.calculate_memory_score(0.8, 0.8, 50.0);
        
        assert!(score2 > score1);
        assert!(score1 >= 0.0 && score1 <= 1.0);
        assert!(score2 >= 0.0 && score2 <= 1.0);
    }
    
    // Additional tests would be implemented here
}