//! Packing view for the Flux GUI
//! This module contains the action types for the packing view

/// Actions that can be triggered from the packing view
#[derive(Debug, Clone)]
pub enum PackingAction {
    /// Remove a file at the given index
    RemoveFile(usize),
    /// Select output location
    SelectOutput,
    /// Add more files to pack
    AddMoreFiles,
    /// Start the packing operation
    StartPacking,
    /// Clear all selections
    ClearAll,
    /// Cancel the current operation
    Cancel,
}