//! Interactive mode support

use std::path::Path;

/// Conflict resolution action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictAction {
    /// Overwrite the existing file
    Overwrite,
    /// Skip this file
    Skip,
    /// Rename the file
    Rename,
    /// Overwrite all subsequent conflicts
    OverwriteAll,
    /// Skip all subsequent conflicts
    SkipAll,
    /// Quit the operation
    Quit,
}

/// Interactive conflict resolver trait
pub trait ConflictResolver {
    /// Resolve a file conflict
    fn resolve_conflict(&mut self, path: &Path) -> ConflictAction;
}

/// Non-interactive conflict resolver that uses a fixed action
pub struct FixedResolver {
    action: ConflictAction,
}

impl FixedResolver {
    /// Create a resolver that always overwrites
    pub fn overwrite() -> Self {
        Self {
            action: ConflictAction::Overwrite,
        }
    }
    
    /// Create a resolver that always skips
    pub fn skip() -> Self {
        Self {
            action: ConflictAction::Skip,
        }
    }
    
    /// Create a resolver that always renames
    pub fn rename() -> Self {
        Self {
            action: ConflictAction::Rename,
        }
    }
}

impl ConflictResolver for FixedResolver {
    fn resolve_conflict(&mut self, _path: &Path) -> ConflictAction {
        self.action
    }
}

/// State-tracking resolver that can remember "all" decisions
pub struct StatefulResolver<R: ConflictResolver> {
    inner: R,
    override_action: Option<ConflictAction>,
}

impl<R: ConflictResolver> StatefulResolver<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            override_action: None,
        }
    }
}

impl<R: ConflictResolver> ConflictResolver for StatefulResolver<R> {
    fn resolve_conflict(&mut self, path: &Path) -> ConflictAction {
        if let Some(action) = self.override_action {
            return action;
        }
        
        let action = self.inner.resolve_conflict(path);
        
        match action {
            ConflictAction::OverwriteAll => {
                self.override_action = Some(ConflictAction::Overwrite);
                ConflictAction::Overwrite
            }
            ConflictAction::SkipAll => {
                self.override_action = Some(ConflictAction::Skip);
                ConflictAction::Skip
            }
            _ => action,
        }
    }
}