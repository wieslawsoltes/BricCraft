//! Native studio composition; acquisition and conversion stay outside the UI crate.
use crate::{controls, import_area, import_model, theme, Dispatch, UiAction, UiState};
use ferroui_base::media::TextWrapping;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

pub fn panel(state: UiState, dispatch: Dispatch) -> Element {
    let report = dispatch.clone();
    let places = dispatch.clone();
    let content = scroll_viewer()
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(
            stack_panel().spacing(16.0).children((
                button()
                    .name("StudioPlaces")
                    .class("primary")
                    .content("Choose a place on the map")
                    .on_click(move |_| places(UiAction::ShowPlaces)),
                import_area::view(state, dispatch.clone()),
                border()
                    .height(1.0)
                    .background(theme::EDGE)
                    .margin((0.0, 5.0)),
                import_model::view(state, dispatch),
                text_block()
                    .text(state.import_status)
                    .foreground(theme::MUTED)
                    .font_size(12.0)
                    .text_wrapping(TextWrapping::Wrap),
                button()
                    .class("quiet")
                    .content("Export this world's source report")
                    .on_click(move |_| report(UiAction::ExportReport)),
            )),
        )
        .into();
    border()
        .is_visible(state.imports_open)
        .child(controls::panel(
            "World studio",
            "REAL PLACES. REAL GEOMETRY.",
            content,
            move || state.imports_open.set(false),
            state,
        ))
        .into()
}
