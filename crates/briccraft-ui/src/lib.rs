//! Native, declarative FerroUI interface for reusable brick-world simulations.
//! No DOM widgets, generated markup, canvas-painted controls or imperative trees.
mod catalog;
mod controls;
mod dialogs;
mod hotbar;
mod hud;
mod model;
mod preview;
mod theme;

use ferroui_view::prelude::*;
pub use model::{ColorSwatch, Dispatch, PartCard, UiAction, UiState, HOTBAR_PARTS};
use std::rc::Rc;

/// Compose the world visual with reactive native UI. The world is mounted once;
/// only the small components that read a changed signal run again.
pub fn view(
    world: Element,
    parts: Rc<Vec<PartCard>>,
    colors: Rc<Vec<ColorSwatch>>,
    state: UiState,
    dispatch: Dispatch,
) -> Element {
    grid()
        .name("BricCraftRoot")
        .styles(theme::styles())
        .children((
            world,
            hud::header(state, dispatch.clone()),
            hud::world_card(state, dispatch.clone()),
            hud::crosshair(state),
            hotbar::hotbar(parts.clone(), colors.clone(), state, dispatch.clone()),
            hud::footer(state, dispatch.clone()),
            catalog::catalog(parts, colors, state, dispatch.clone()),
            dialogs::blueprints(state, dispatch.clone()),
            dialogs::settings(state, dispatch.clone()),
            dialogs::help(state),
            controls::touch_controls(state, dispatch),
        ))
        .into()
}
