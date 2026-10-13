//! The egui UI: three panes (sidebar, main list, inspector) with the Nucleus
//! dark theme.

pub mod theme;
pub mod views;
pub mod widgets;

pub use theme::configure_theme;
pub use views::{Action, Counts, Facts, Filter, NodeFact};
