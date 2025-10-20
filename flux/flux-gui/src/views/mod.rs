//! View modules for Flux GUI

pub mod packing_view;
pub mod extracting_view;

pub use packing_view::{draw_packing_view, PackingAction};
pub use extracting_view::{draw_extracting_view, ExtractingAction};