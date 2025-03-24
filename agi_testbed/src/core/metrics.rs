use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use chrono::{DateTime, Utc};
use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use std::time::Duration;

use crate::core::component::{Component, ComponentMetrics, ComponentError};

/// Manager for collecting and storing metrics from all components
#[derive(Debug)]
pub struct MetricsManager {
    /// Historical metrics data
    metrics_history: Arc<Mutex<HashMap<String, Vec<ComponentMetrics>>>>,
    /// Most recent metrics for each component
    latest_metrics: Arc<RwLock<HashMap<String, ComponentMetrics>>>,
    /// Configuration for metrics collection
    config: MetricsConfig,
}

/// Configuration for metrics collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Collection interval in seconds
    pub collection_interval_secs: u64,
    /// Maximum history to keep per component
    pub max_history_per_component: usize,
    /// Enable detailed metrics
    pub detailed_metrics: bool,
    /// Metrics storage path
    pub storage_path: Option<String>,
}

/// Summary metrics for the entire system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetricsSummary {
    /// Timestamp of the summary
    pub timestamp: DateTime<Utc>,
    /// Total CPU usage across all components (percentage)
    pub total_cpu_usage: f64,
    /// Total memory usage across all components (MB)
    pub total_memory_usage: f64,
    /// Average task processing time (ms)
    pub avg_processing_time: f64,
    /// Total tasks processed
    pub total_tasks_processed: u64,
    /// Component-specific highlights
    pub component_highlights: HashMap<String, ComponentMetricHighlight>,
}

/// Highlight metrics for a specific component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentMetricHighlight {
    /// Component identifier
    pub component_id: String,
    /// CPU usage (percentage)
    pub cpu_usage: f64,
    /// Memory usage (MB)
    pub memory_usage: f64,
    /// Tasks processed
    pub tasks_processed: u64,
    /// Key performance indicator
    pub main_kpi: f64,
    /// KPI label
    pub main_kpi_label: String,
}

/// Time series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricTimeSeriesPoint {
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Metric value
    pub value: f64,
}

/// Time series data for a specific metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricTimeSeries {
    /// Metric name
    pub name: String,
    /// Component ID
    pub component_id: String,
    /// Data points
    pub data_points: Vec<MetricTimeSeriesPoint>,
    /// Unit of measurement
    pub unit: String,
}

impl MetricsManager {
    /// Create a new metrics manager with the specified configuration
    pub fn new(config: MetricsConfig) -> Self {
        Self {
            metrics_history: Arc::new(Mutex::new(HashMap::new())),
            latest_metrics: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Start the metrics collection process
    pub async fn start_collection(
        &self,
        components: HashMap<String, Arc<RwLock<dyn Component>>>,
    ) -> Result<(), ComponentError> {
        info!("Starting metrics collection process");
        
        let metrics_history = self.metrics_history.clone();
        let latest_metrics = self.latest_metrics.clone();
        let interval = self.config.collection_interval_secs;
        let max_history = self.config.max_history_per_component;
        let components = components.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval));
            
            loop {
                interval.tick().await;
                debug!("Collecting metrics from all components");
                
                for (component_id, component) in &components {
                    match component.read().await.collect_metrics().await {
                        Ok(metrics) => {
                            // Update latest metrics
                            latest_metrics.write().await.insert(component_id.clone(), metrics.clone());
                            
                            // Update historical metrics
                            let mut history = metrics_history.lock().await;
                            let component_history = history.entry(component_id.clone())
                                .or_insert_with(Vec::new);
                            
                            component_history.push(metrics);
                            
                            // Trim history if needed
                            if component_history.len() > max_history {
                                let excess = component_history.len() - max_history;
                                component_history.drain(0..excess);
                            }
                        },
                        Err(e) => {
                            error!("Failed to collect metrics from {}: {:?}", component_id, e);
                        }
                    }
                }
                
                // Optionally, we could save metrics to disk here
            }
        });
        
        Ok(())
    }
    
    /// Get the latest metrics for a specific component
    pub async fn get_latest_component_metrics(&self, component_id: &str) -> Option<ComponentMetrics> {
        self.latest_metrics.read().await.get(component_id).cloned()
    }
    
    /// Get historical metrics for a specific component
    pub async fn get_component_history(
        &self,
        component_id: &str,
        limit: Option<usize>,
    ) -> Vec<ComponentMetrics> {
        let history = self.metrics_history.lock().await;
        
        if let Some(component_history) = history.get(component_id) {
            let limit = limit.unwrap_or(component_history.len());
            if limit >= component_history.len() {
                return component_history.clone();
            } else {
                return component_history.iter()
                    .skip(component_history.len() - limit)
                    .cloned()
                    .collect();
            }
        }
        
        Vec::new()
    }
    
    /// Get a system-wide metrics summary
    pub async fn get_system_summary(&self) -> SystemMetricsSummary {
        let latest = self.latest_metrics.read().await;
        
        let now = Utc::now();
        let mut total_cpu = 0.0;
        let mut total_memory = 0.0;
        let mut total_processing_time = 0.0;
        let mut total_tasks = 0;
        let mut processing_time_samples = 0;
        let mut component_highlights = HashMap::new();
        
        for (id, metrics) in latest.iter() {
            total_cpu += metrics.cpu_usage;
            total_memory += metrics.memory_usage;
            total_processing_time += metrics.avg_processing_time;
            total_tasks += metrics.tasks_processed as usize;
            processing_time_samples += 1;
            
            // Create a component highlight with a default KPI
            component_highlights.insert(id.clone(), ComponentMetricHighlight {
                component_id: id.clone(),
                cpu_usage: metrics.cpu_usage,
                memory_usage: metrics.memory_usage,
                tasks_processed: metrics.tasks_processed,
                main_kpi: metrics.avg_processing_time,
                main_kpi_label: "Avg. Processing Time (ms)".to_string(),
            });
        }
        
        SystemMetricsSummary {
            timestamp: now,
            total_cpu_usage: total_cpu,
            total_memory_usage: total_memory,
            avg_processing_time: if processing_time_samples > 0 {
                total_processing_time / processing_time_samples as f64
            } else {
                0.0
            },
            total_tasks_processed: total_tasks as u64,
            component_highlights,
        }
    }
    
    /// Get time series data for a specific metric
    pub async fn get_metric_time_series(
        &self,
        component_id: &str,
        metric_name: &str,
        limit: Option<usize>,
    ) -> Result<MetricTimeSeries, ComponentError> {
        let history = self.get_component_history(component_id, limit).await;
        
        let mut time_series = MetricTimeSeries {
            name: metric_name.to_string(),
            component_id: component_id.to_string(),
            data_points: Vec::new(),
            unit: "".to_string(),
        };
        
        // Extract the requested metric from component history
        for metrics in history {
            let value = match metric_name {
                "cpu_usage" => Some((metrics.cpu_usage, "%")),
                "memory_usage" => Some((metrics.memory_usage, "MB")),
                "tasks_processed" => Some((metrics.tasks_processed as f64, "count")),
                "avg_processing_time" => Some((metrics.avg_processing_time, "ms")),
                _ => {
                    // Check for custom metrics in the JSON value
                    if let Some(custom_value) = metrics.custom_metrics.get(metric_name) {
                        if let Some(num) = custom_value.as_f64() {
                            Some((num, ""))
                        } else if let Some(num) = custom_value.as_i64() {
                            Some((num as f64, ""))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
            };
            
            if let Some((value, unit)) = value {
                time_series.data_points.push(MetricTimeSeriesPoint {
                    timestamp: metrics.timestamp,
                    value,
                });
                
                if time_series.unit.is_empty() {
                    time_series.unit = unit.to_string();
                }
            }
        }
        
        if time_series.data_points.is_empty() {
            return Err(ComponentError::ValidationError(format!(
                "Metric '{}' not found for component '{}'", metric_name, component_id
            )));
        }
        
        Ok(time_series)
    }
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            collection_interval_secs: 10,
            max_history_per_component: 1000,
            detailed_metrics: true,
            storage_path: None,
        }
    }
}

// Tests for MetricsManager
#[cfg(test)]
mod tests {
    use super::*;
    
    // Test creation of a new metrics manager
    #[test]
    fn test_new_metrics_manager() {
        let config = MetricsConfig::default();
        let manager = MetricsManager::new(config);
        
        // Basic assertion to verify instantiation
        assert_eq!(manager.config.collection_interval_secs, 10);
    }
    
    // Additional tests would be implemented here
}