//! Readiness is the same assessment used by actual vehicle activation.
use crate::{controls, theme, Dispatch, UiAction, UiState};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;

pub fn view(state: UiState, dispatch: Dispatch) -> Element {
    border()
        .name("WorkshopReadiness")
        .background(theme::INSET)
        .corner_radius(10.0)
        .padding(14.0)
        .child(
            stack_panel().spacing(10.0).children((
                controls::label("CONSTRUCTION WORKSHOP"),
                text_block()
                    .text(move || {
                        if state.mode.get() == "DRIVE" {
                            "Construction in motion".into()
                        } else {
                            state.gameplay.get().workshop.title
                        }
                    })
                    .font_size(14.0)
                    .font_weight(FontWeight::SemiBold)
                    .foreground(move || {
                        if state.gameplay.get().workshop.ready || state.mode.get() == "DRIVE" {
                            "#AFE5BC"
                        } else {
                            "#F2BD7B"
                        }
                    })
                    .text_wrapping(TextWrapping::Wrap),
                text_block()
                    .text(move || {
                        if state.mode.get() == "DRIVE" {
                            state.mechanics.get()
                        } else {
                            state.gameplay.get().workshop.detail
                        }
                    })
                    .font_size(12.0)
                    .foreground(theme::MUTED)
                    .text_wrapping(TextWrapping::Wrap),
                button()
                    .name("WorkshopDrive")
                    .class("primary")
                    .content(move || {
                        if state.mode.get() == "DRIVE" {
                            "Park & exit"
                        } else {
                            "Drive construction"
                        }
                    })
                    .is_enabled(move || {
                        state.mode.get() == "DRIVE" || state.gameplay.get().workshop.ready
                    })
                    .on_click(move |_| {
                        dispatch(if state.mode.get_untracked() == "DRIVE" {
                            UiAction::ExitVehicle
                        } else {
                            UiAction::ActivateVehicle("auto".into())
                        })
                    }),
                button()
                    .content("Open vehicle blueprints")
                    .on_click(move |_| {
                        state.close_panels();
                        state.blueprints_open.set(true);
                    }),
            )),
        )
        .into()
}
