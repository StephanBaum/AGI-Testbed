//! Operational State Instance Module
//! 
//! This module implements the Persistent Operational State capability,
//! which demonstrates autonomous operation in multiple modes:
//! - Problem-solving
//! - Exploration
//! - Consolidation
//! - Planning
//! 
//! The instance balances resources among concurrent tasks and maintains
//! continuous operation with various priorities.

mod models;
mod instance;
mod task;
mod goal;
mod resource;
mod mode;

// Re-export main components
pub use instance::OperationalStateInstance;
pub use models::{
    OperationalMode, OperationalModeConfig, TaskStatus, GoalStatus, RecoveryState
};