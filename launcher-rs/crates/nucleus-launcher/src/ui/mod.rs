//! The egui UI: header + scrollable card grid / table + footer, in the
//! craft-launcher spirit but with the Nucleus tokens and branding.

pub mod icons;
pub mod theme;
pub mod views;
pub mod widgets;

pub use theme::{Palette, configure_theme, install_fonts};
pub use views::{Action, Counts, Facts, NodeFact, SortKey, View, ViewState};
