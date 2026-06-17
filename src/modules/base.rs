//! Memory Module Base - Base trait for all memory modules
//!
//! Corresponds to MemoryModuleBase in C# implementation

use anyhow::Result;

/// Base trait for memory modules
pub trait MemoryModuleBase {
    /// Get the shared memory name
    fn get_memory_name(&self) -> &str;

    /// Initialize the module
    fn initialize(&mut self) -> Result<()>;

    /// Shutdown the module
    fn shutdown(&mut self);

    /// Check if module is running
    fn is_running(&self) -> bool;
}
