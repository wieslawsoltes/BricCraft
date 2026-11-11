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
        .width(move || if state.compact.get() { 284.0 } else { 370.0 })
        .horizontal_alignment(H::Right)
        .vertical_alignment(V::Stretch)
        .margin(move || {
            if state.compact.get() {
                (12.0, 86.0, 12.0, 52.0)
            } else {
                (16.0, 90.0, 20.0, 150.0)
            }
        })
        .background(theme::SURFACE)
        .border_brush(theme::EDGE)
        .border_thickness(1.0)
        .corner_radius(16.0)
        .padding(move || if state.compact.get() { 14.0 } else { 20.0 })
        .effect(theme::shadow())
        .child(
            grid().row_definitions("Auto,*").children((
                grid()
                    .column_definitions("*,Auto")
                    .margin((0.0, 0.0, 0.0, 18.0))
                    .children((
                        stack_panel().spacing(5.0).children((
                            label(eyebrow),
                            text_block()
                                .text(title)
                                .font_size(24.0)
                                .font_weight(FontWeight::SemiBold),
                        )),
                        button()
                            .grid_column(1)
                            .class("quiet")
                            .content("×")
                            .font_size(23.0)
                            .width(36.0)
                            .height(36.0)
                            .padding(0.0)
                            .tool_tip_tip("Close panel · Esc")
                            .on_click(move |_| close()),
                    )),
                border().grid_row(1).child(content),
            )),
        )
        .into()
}

pub fn touch_controls(state: UiState, dispatch: Dispatch) -> Element {
    let forward = dispatch.clone();
    let back = dispatch.clone();
    let left = dispatch.clone();
    let right = dispatch.clone();
    let up = dispatch.clone();
    let place = dispatch.clone();
    let remove = dispatch;
    grid()
        .name("TouchControls")
        .is_visible(state.compact)
        .horizontal_alignment(H::Left)
        .vertical_alignment(V::Bottom)
        .margin((16.0, 0.0, 16.0, 158.0))
        .column_definitions("Auto,Auto,Auto,20,Auto")
        .row_definitions("Auto,Auto")
        .children((
            button()
                .grid_column(1)
                .content("↑")
                .width(42.0)
                .height(42.0)
                .tool_tip_tip("Move forward")
                .on_click(move |_| {
                    forward(UiAction::Move {
                        forward: 1.0,
                        strafe: 0.0,
                        up: 0.0,
                    })
                }),
            button()
                .grid_row(1)
                .content("←")
                .width(42.0)
                .height(42.0)
                .tool_tip_tip("Move left")
                .on_click(move |_| {
                    left(UiAction::Move {
                        forward: 0.0,
                        strafe: -1.0,
                        up: 0.0,
                    })
                }),
            button()
                .grid_row(1)
                .grid_column(1)
                .content("↓")
                .width(42.0)
                .height(42.0)
                .tool_tip_tip("Move backward")
                .on_click(move |_| {
                    back(UiAction::Move {
                        forward: -1.0,
                        strafe: 0.0,
                        up: 0.0,
                    })
                }),
            button()
                .grid_row(1)
                .grid_column(2)
                .content("→")
                .width(42.0)
                .height(42.0)
                .tool_tip_tip("Move right")
                .on_click(move |_| {
                    right(UiAction::Move {
                        forward: 0.0,
                        strafe: 1.0,
                        up: 0.0,
                    })
                }),
            button()
                .grid_column(2)
                .content("↟")
                .width(42.0)
                .height(42.0)
                .tool_tip_tip("Jump / rise")
                .on_click(move |_| {
                    up(UiAction::Move {
                        forward: 0.0,
                        strafe: 0.0,
                        up: 1.0,
                    })
                }),
            button()
                .grid_column(4)
                .class("primary")
                .content("+")
                .width(42.0)
                .height(42.0)
                .tool_tip_tip("Place selected part")
                .on_click(move |_| place(UiAction::Place)),
            button()
                .grid_column(4)
                .grid_row(1)
                .content("−")
                .width(42.0)
                .height(42.0)
                .tool_tip_tip("Remove target part")
                .on_click(move |_| remove(UiAction::Remove)),
        ))
        .into()
}
