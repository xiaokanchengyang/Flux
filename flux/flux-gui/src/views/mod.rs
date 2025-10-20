//! View modules for Flux GUI

pub mod packing_view;
pub mod extracting_view;
pub mod sync_view;
pub mod packing_view_modern;

pub use packing_view::PackingAction;
pub use extracting_view::{draw_extracting_view, ExtractingAction};
pub use sync_view::{draw_sync_view, SyncAction};
pub use packing_view_modern::draw_packing_view_modern;