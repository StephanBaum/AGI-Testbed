//! Component instances for the AGI Testbed
//!
//! This module contains the specialized instances that implement the core
//! capabilities of the AGI architecture.

pub mod common;
pub mod time_scaling;
pub mod memory_management;
pub mod reasoning;
pub mod operational_state;

// Re-export instances for convenience
pub use time_scaling::TimeScalingInstance;
pub use memory_management::MemoryManagementInstance;
pub use reasoning::ReasoningInstance;
pub use operational_state::OperationalStateInstance;