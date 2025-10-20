//! Interactive mode implementation for CLI

use dialoguer::Select;
use flux_core::interactive::{ConflictAction, ConflictResolver};
use std::path::Path;

/// Interactive CLI conflict resolver
pub struct CliInteractiveResolver;

impl ConflictResolver for CliInteractiveResolver {
    fn resolve_conflict(&mut self, path: &Path) -> ConflictAction {
        let options = vec![
            "[O]verwrite",
            "[S]kip", 
            "[R]ename",
            "[A]llow all (overwrite all)",
            "[N]o to all (skip all)",
            "[Q]uit",
        ];
        
        let prompt = format!("File already exists: {:?}", path);
        let selection = Select::new()
            .with_prompt(&prompt)
            .items(&options)
            .default(1) // Default to Skip
            .interact()
            .unwrap_or(1);
            
        match selection {
            0 => ConflictAction::Overwrite,
            1 => ConflictAction::Skip,
            2 => ConflictAction::Rename,
            3 => ConflictAction::OverwriteAll,
            4 => ConflictAction::SkipAll,
            5 => ConflictAction::Quit,
            _ => ConflictAction::Skip,
        }
    }
}