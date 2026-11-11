//! Work stays observable and cancellable while source conversion runs off-thread.
use crate::{controls, theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

pub fn view(state: UiState, dispatch: Dispatch) -> Element {
    border()
        .name("ImportProgress")
        .is_visible(move || state.import_busy.get() || state.world_busy.get())
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Center)
        .width(move || (state.layout.get().width - 32.0).clamp(1.0, 440.0))
        .max_height(move || (state.layout.get().height - 32.0).max(1.0))
        .padding(move || if state.compact.get() { 20.0 } else { 26.0 })
        .corner_radius(18.0)
        .background(theme::SURFACE)
        .border_brush(theme::EDGE)
        .border_thickness(1.0)
        .effect(theme::shadow())
        .child(
            scroll_viewer()
                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                .child(
                    stack_panel().spacing(16.0).children((
                        controls::label("EVERY PIECE HAS A PLACE"),
                        text_block()
                            .text(move || {
                                if state.world_busy.get() {
                                    "Opening your world"
                                } else {
                                    "Building your world"
                                }
                            })
                            .font_size(24.0)
                            .font_weight(FontWeight::SemiBold)
                            .text_wrapping(TextWrapping::Wrap),
                        text_block()
                            .name("ImportStage")
                            .text(move || {
                                if state.world_busy.get() {
                                    state.status.get()
                                } else {
                                    state.import_status.get()
                                }
                            })
                            .font_size(13.0)
                            .foreground(theme::MUTED)
                            .text_wrapping(TextWrapping::Wrap),
                        stack_panel().spacing(8.0).children((
                            progress_bar()
                                .is_indeterminate(state.world_busy)
                                .minimum(0.0)
                                .maximum(1.0)
                                .value(state.import_progress)
                                .height(8.0),
                            text_block()
                                .is_visible(move || !state.world_busy.get())
                                .text(move || {
                                    format!(
                                        "{:.0}% complete",
                                        state.import_progress.get().clamp(0.0, 1.0) * 100.0
                                    )
                                })
                                .font_size(11.0)
                                .foreground(theme::MUTED)
                                .horizontal_alignment(H::Right),
                        )),
                        button()
                            .name("CancelGeneration")
                            .is_visible(state.import_busy)
                            .content("Cancel generation")
                            .on_click(move |_| dispatch(UiAction::CancelImport)),
                    )),
                ),
        )
        .into()
}
