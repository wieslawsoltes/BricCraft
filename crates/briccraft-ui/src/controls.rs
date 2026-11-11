use crate::{theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;

pub fn label(value: impl Into<String>) -> Element {
    text_block()
        .text(value.into())
        .foreground(theme::MUTED)
        .font_size(10.0)
        .font_weight(FontWeight::SemiBold)
        .into()
}

pub fn caption(value: impl Into<String>) -> Element {
    text_block()
        .text(value.into())
        .foreground(theme::MUTED)
        .text_wrapping(TextWrapping::Wrap)
        .font_size(12.0)
        .into()
}

pub fn action_button(title: &str, hint: &str, action: UiAction, dispatch: Dispatch) -> Element {
    button()
        .content(title)
        .tool_tip_tip(hint)
        .on_click(move |_| dispatch(action.clone()))
        .into()
}

pub fn keycap(text: &str) -> Element {
    border()
        .background("#283840")
        .border_brush("#455962")
        .border_thickness((1.0, 1.0, 1.0, 2.0))
        .corner_radius(4.0)
        .padding((6.0, 2.0))
        .child(
            text_block()
                .text(text)
                .font_size(10.0)
                .foreground("#D7E3E7"),
        )
        .into()
}

pub fn panel(
    title: &str,
    eyebrow: &str,
    content: Element,
    close: impl Fn() + 'static,
    state: UiState,
) -> Element {
    border()
        .name(format!("{}Panel", title.replace(' ', "")))
        .width(move || state.layout.get().panel_width())
        .horizontal_alignment(H::Right)
        .vertical_alignment(V::Stretch)
        .margin(move || state.layout.get().panel_margin())
        .background(theme::SURFACE)
        .border_brush(theme::EDGE)
        .border_thickness(1.0)
        .corner_radius(16.0)
        .padding(move || {
            if state.layout.get().landscape {
                12.0
            } else if state.compact.get() {
                16.0
            } else {
                22.0
            }
        })
        .effect(theme::shadow())
        .child(
            grid().row_definitions("Auto,*").children((
                grid()
                    .column_definitions("*,Auto")
                    .margin(move || {
                        (
                            0.0,
                            0.0,
                            0.0,
                            if state.layout.get().landscape {
                                8.0
                            } else {
                                14.0
                            },
                        )
                    })
                    .children((
                        stack_panel().spacing(5.0).children((
                            border()
                                .is_visible(move || !state.layout.get().landscape)
                                .child(label(eyebrow)),
                            text_block()
                                .text(title)
                                .font_size(move || if state.compact.get() { 22.0 } else { 24.0 })
                                .font_weight(FontWeight::SemiBold),
                        )),
                        button()
                            .grid_column(1)
                            .class("quiet")
                            .content("Close")
                            .font_size(11.0)
                            .width(52.0)
                            .height(44.0)
                            .padding(0.0)
                            .tool_tip_tip("Close panel · Esc")
                            .on_click(move |_| close()),
                    )),
                border().grid_row(1).child(content),
            )),
        )
        .into()
}

/// A panel consumes outside taps before the world can interpret them as edits.
pub fn modal_backdrop(state: UiState) -> Element {
    border()
        .name("PanelBackdrop")
        .background("#66071118")
        .is_visible(move || state.modal())
        .on_pointer_pressed(move |event| {
            if !state.import_busy.get_untracked() && !state.world_busy.get_untracked() {
                state.close_panels();
            }
            event.set_handled(true);
        })
        .into()
}
