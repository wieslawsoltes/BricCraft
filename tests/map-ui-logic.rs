//! Exercise the production map math and contact tracking without loading a window backend.
#![allow(dead_code)]
#[path = "../crates/briccraft-ui/src/layout.rs"]
mod layout;
#[path = "../crates/briccraft-ui/src/map_geometry.rs"]
mod map_geometry;
#[path = "../crates/briccraft-ui/src/map_gestures.rs"]
mod map_gestures;
#[path = "../crates/briccraft-ui/src/map_location.rs"]
mod map_location;
#[path = "../crates/briccraft-ui/src/navigation_model.rs"]
mod navigation_model;
pub use layout::UiLayout;
pub use map_location::MapView;
