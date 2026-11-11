use crate::{theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::Button;
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
                (12.0, 118.0, 12.0, 52.0)
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

/// Native Button press state includes pointer release, capture loss, and cancellation.
/// Each axis combines its two directions, so lifting one finger never clears a
/// different finger's steering or throttle input.
fn held_button(
    title: &str,
    hint: &str,
    axis: u8,
    positive: bool,
    held: Signal<[bool; 6]>,
    dispatch: Dispatch,
) -> El<Button> {
    button()
        .content(title)
        .width(42.0)
        .height(42.0)
        .padding(0.0)
        .font_size(11.0)
        .focusable(false)
        .tool_tip_tip(hint)
        .on_changed(Button::is_pressed_property(), move |pressed| {
            let mut buttons = held.get_untracked();
            let offset = axis as usize * 2;
            buttons[offset + usize::from(positive)] = *pressed;
            held.set(buttons);
            dispatch(UiAction::TouchAxis {
                axis,
                value: u8::from(buttons[offset + 1]) as f64 - u8::from(buttons[offset]) as f64,
            });
        })
}

pub fn touch_controls(state: UiState, dispatch: Dispatch) -> Element {
    let held = signal([false; 6]);
    let place = dispatch.clone();
    let remove = dispatch.clone();
    grid()
        .name("TouchControls")
        .is_visible(move || state.compact.get() && !state.modal())
        .horizontal_alignment(H::Left)
        .vertical_alignment(V::Bottom)
        .margin((16.0, 0.0, 16.0, 158.0))
        .column_definitions("Auto,Auto,Auto,20,Auto")
        .row_definitions("Auto,Auto")
        .children((
            held_button("Down", "Descend", 1, false, held, dispatch.clone()),
            held_button(
                "↑",
                "Move forward / accelerate",
                2,
                true,
                held,
                dispatch.clone(),
            )
            .grid_column(1),
            held_button("Up", "Jump / rise", 1, true, held, dispatch.clone()).grid_column(2),
            held_button(
                "←",
                "Move left / steer left",
                0,
                false,
                held,
                dispatch.clone(),
            )
            .grid_row(1),
            held_button(
                "↓",
                "Move backward / reverse",
                2,
                false,
                held,
                dispatch.clone(),
            )
            .grid_row(1)
            .grid_column(1),
            held_button("→", "Move right / steer right", 0, true, held, dispatch)
                .grid_row(1)
                .grid_column(2),
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
