//! Native, declarative FerroUI interface for reusable brick-world simulations.
//! No DOM widgets, generated markup, canvas-painted controls or imperative trees.
mod blueprint_preview;
mod blueprints;
mod catalog;
mod controls;
mod credits;
mod design_model;
mod designs;
mod dialogs;
mod gameplay;
mod gameplay_model;
mod header;
mod help;
mod hotbar;
mod hud;
mod icons;
mod import_area;
mod import_model;
mod import_progress;
mod imports;
mod layout;
mod model;
mod objectives;
mod preview;
mod settings;
mod theme;
mod touch;
mod workshop;
mod world_details;
mod worlds;

pub use design_model::{DesignLibraryView, SavedDesignCard};
use ferroui_view::prelude::*;
pub use gameplay_model::{BlueprintQuote, GameplayView, ObjectiveCard, WorkshopView};
pub use layout::UiLayout;
pub use model::{
    ColorSwatch, Dispatch, PartCard, SourceCredit, UiAction, UiState, WorldSummary, HOTBAR_PARTS,
};
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
        .children(vec![
            world,
            grid()
                .name("InterfaceLayer")
                .margin(move || state.layout.get().safe_margin())
                .clip_to_bounds(true)
                .children(vec![
                    header::view(state, dispatch.clone()),
                    hud::world_card(state, dispatch.clone()),
                    hud::crosshair(state),
                    hotbar::hotbar(parts.clone(), colors.clone(), state, dispatch.clone()),
                    hud::footer(state, dispatch.clone()),
                    hud::activity(state),
                    credits::badge(state),
                    touch::controls(state, dispatch.clone()),
                    controls::modal_backdrop(state),
                    catalog::catalog(parts, colors, state, dispatch.clone()),
                    dialogs::blueprints(state, dispatch.clone()),
                    dialogs::settings(state, dispatch.clone()),
                    dialogs::help(state),
                    worlds::panel(state, dispatch.clone()),
                    imports::panel(state, dispatch.clone()),
                    credits::panel(state, dispatch.clone()),
                    gameplay::panel(state, dispatch.clone()),
                    world_details::panel(state),
                    import_progress::view(state, dispatch),
                ])
                .into(),
        ])
        .into()
}
