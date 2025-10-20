//! Tokio runtime management for synchronous operations

use std::sync::Arc;
use tokio::runtime::Runtime;
use std::sync::OnceLock;

/// Get or create a shared Tokio runtime for blocking operations
pub(crate) fn get_runtime() -> Arc<Runtime> {
    static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
    
    RUNTIME.get_or_init(|| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("flux-cloud-worker")
            .build()
            .expect("Failed to create Tokio runtime");
        
        Arc::new(runtime)
    }).clone()
}