//! Map-first place selection with a responsive native viewport and generation controls.
use crate::{
    controls, icons::Icon, map_geometry as geo, map_viewport, place_options, theme, Dispatch,
    UiAction, UiState,
};
use ferroui_base::{
    layout::{HorizontalAlignment as H, VerticalAlignment as V},
    media::{FontWeight, TextWrapping},
};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

fn map(state: UiState, dispatch: Dispatch) -> Element {
    let retry = dispatch.clone();
    stack_panel()
        .spacing(8.0)
        .children((
            border()
                .corner_radius(12.0)
                .clip_to_bounds(true)
                .child(map_viewport::view(state, dispatch)),
            grid().column_definitions("*,Auto").children((
                stack_panel()
                    .spacing(3.0)
                    .vertical_alignment(V::Center)
                    .margin((0.0, 0.0, 6.0, 0.0))
                    .children((
                        text_block()
                            .text("Drag to explore · scroll or pinch to zoom · tap to choose")
                            .font_size(11.0)
                            .foreground(theme::MUTED)
                            .text_wrapping(TextWrapping::Wrap),
                        text_block()
                            .name("MapLoadStatus")
                            .text(state.map_status)
                            .font_size(11.0)
                            .foreground(theme::POSITIVE)
                            .text_wrapping(TextWrapping::Wrap),
                    )),
                controls::tool_button(
                    "ReloadMap",
                    "Reload visible map tiles",
                    Icon::Redo,
                    move || retry(UiAction::MapViewportChanged),
                )
                .grid_column(1),
            )),
        ))
        .into()
}

pub fn roam_card(state: UiState, dispatch: Dispatch) -> Element {
    button()
        .name("RoamStatus")
        .is_visible(move || !state.roam_status.get().is_empty())
        .background(theme::HUD)
        .border_brush("#597F79")
        .padding((12.0, 10.0))
        .horizontal_content_alignment(H::Stretch)
        .tool_tip_tip("Open live map status, resume or pause streaming")
        .on_click(move |_| dispatch(UiAction::ShowPlaces))
        .child(
            stack_panel().spacing(5.0).children((
                text_block()
                    .text(move || {
                        if state.roam_active.get() {
                            "FREE ROAM · LIVE MAP"
                        } else {
                            "MAP STREAM"
                        }
                    })
                    .font_size(10.0)
                    .foreground(theme::MUTED)
                    .font_weight(FontWeight::SemiBold),
                crate::map_navigation::summary(state),
                text_block()
                    .text(state.roam_status)
                    .font_size(11.0)
                    .foreground(theme::POSITIVE)
                    .text_wrapping(TextWrapping::Wrap)
                    .max_lines(3),
            )),
        )
        .into()
}

fn content(state: UiState, dispatch: Dispatch) -> Element {
    // Only a structural breakpoint changes this tree. Child sizes already
    // follow layout signals; resizing within a mode should retain editors and
    // map interaction state instead of rebuilding them for every measurement.
    let wide = memo(move |_| geo::map_wide(state.layout.get()));
    rerender(move || {
        if wide.get() {
            grid()
                .column_definitions("*,300")
                .children((
                    border()
                        .margin((0.0, 0.0, 16.0, 0.0))
                        .child(map(state, dispatch.clone())),
                    scroll_viewer()
                        .grid_column(1)
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .child(place_options::view(state, dispatch.clone())),
                ))
                .into_element()
        } else {
            scroll_viewer()
                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                .child(stack_panel().spacing(16.0).children((
                    map(state, dispatch.clone()),
                    place_options::view(state, dispatch.clone()),
                )))
                .into_element()
        }
    })
}

pub fn panel(state: UiState, dispatch: Dispatch) -> Element {
    border()
        .name("PlacesPanel")
        .is_visible(state.places_open)
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Stretch)
        .width(move || geo::map_panel_width(state.layout.get()))
        .margin(12.0)
        .padding(16.0)
        .background(theme::SURFACE)
        .border_brush("#53697F")
        .border_thickness(1.0)
        .corner_radius(20.0)
        .effect(theme::shadow())
        .child(
            grid().row_definitions("Auto,*").children((
                grid()
                    .column_definitions("*,Auto")
                    .margin((0.0, 0.0, 0.0, 14.0))
                    .children((
                        stack_panel().spacing(4.0).children((
                            controls::label("THE WORLD, READY FOR YOUR NEXT BUILD"),
                            text_block()
                                .text("Choose your place")
                                .font_size(24.0)
                                .font_weight(FontWeight::SemiBold),
                        )),
                        controls::tool_button(
                            "ClosePlaces",
                            "Close places · Esc",
                            Icon::Close,
                            move || state.places_open.set(false),
                        )
                        .grid_column(1)
                        .vertical_alignment(V::Center)
                        .background("#23364A"),
                    )),
                border().grid_row(1).child(content(state, dispatch)),
            )),
        )
        .into()
}
