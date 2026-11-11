//! The complete activity message remains one tap away from the compact HUD.
use crate::{controls, theme, UiState};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

pub fn toggle(state: UiState) {
    let open = !state.world_details_open.get_untracked();
    state.close_panels();
    state.world_details_open.set(open);
}

pub fn panel(state: UiState) -> Element {
    let content = scroll_viewer()
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(
            stack_panel().spacing(16.0).children((
                border()
                    .background(theme::INSET)
                    .corner_radius(14.0)
                    .padding(16.0)
                    .child(
                        stack_panel().spacing(9.0).children((
                            controls::label("LATEST ACTION"),
                            text_block()
                                .text(state.status)
                                .font_size(14.0)
                                .text_wrapping(TextWrapping::Wrap),
                        )),
                    ),
                controls::label("CURRENT WORLD"),
                text_block()
                    .text(state.world_name)
                    .font_size(19.0)
                    .font_weight(FontWeight::SemiBold)
                    .text_wrapping(TextWrapping::Wrap),
                text_block()
                    .text(state.stats)
                    .foreground(theme::MUTED)
                    .text_wrapping(TextWrapping::Wrap),
                text_block()
                    .text(state.coordinates)
                    .foreground(theme::MUTED)
                    .is_visible(state.streamed)
                    .text_wrapping(TextWrapping::Wrap),
                text_block()
                    .text(move || format!("Seed {}", state.seed.get()))
                    .font_size(12.0)
                    .foreground(theme::MUTED),
                controls::label("CONSTRUCTION WORKSHOP"),
                text_block()
                    .text(state.mechanics)
                    .text_wrapping(TextWrapping::Wrap)
                    .foreground(theme::MUTED),
                button()
                    .content("Open objectives & workshop")
                    .on_click(move |_| crate::gameplay::toggle(state)),
                button()
                    .content("View source credits")
                    .is_visible(move || !state.credits.get().is_empty())
                    .on_click(move |_| {
                        state.close_panels();
                        state.credits_open.set(true);
                    }),
            )),
        )
        .into();
    border()
        .is_visible(state.world_details_open)
        .child(controls::panel(
            "World details",
            "YOUR ADVENTURE, AT A GLANCE",
            content,
            move || state.world_details_open.set(false),
            state,
        ))
        .into()
}
