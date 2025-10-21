//! View modules for Flux GUI

pub mod browser_table_view;
pub mod browser_view;
pub mod extracting_view;
pub mod packing_view;
pub mod packing_view_modern;
pub mod sync_view;

pub use browser_view::{draw_browser_view, BrowserAction, BrowserState};
pub use extracting_view::{draw_extracting_view, ExtractingAction};
pub use packing_view::PackingAction;
pub use packing_view_modern::{draw_packing_view_modern, PackingViewContext};
pub use sync_view::{draw_sync_view, SyncAction};
