//! Thumb controls are real native buttons, with independent held axes.
use crate::{theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::Geometry;
use ferroui_controls::Button;
use ferroui_view::prelude::*;

fn held_button(
    name: &str,
    title: &str,
    hint: &str,
    axis: u8,
    positive: bool,
    held: Signal<[bool; 8]>,
    dispatch: Dispatch,
) -> El<Button> {
    let index = axis as usize * 2 + usize::from(positive);
    button()
        .name(name)
        .content(title)
        .width(48.0)
        .height(48.0)
        .margin(2.0)
        .padding(0.0)
        .font_size(11.0)
        .focusable(false)
        .automation_properties_name(Some(hint.to_string()))
        .tool_tip_tip(hint)
        .background(move || {
            if held.get()[index] {
                "#725237"
            } else {
                "#273B46"
            }
        })
        .border_brush(move || {
            if held.get()[index] {
                theme::ACCENT
            } else {
                "#58707B"
            }
        })
        .on_changed(Button::is_pressed_property(), move |pressed| {
            let mut buttons = held.get_untracked();
            buttons[index] = *pressed;
            held.set(buttons);
            let offset = axis as usize * 2;
            dispatch(UiAction::TouchAxis {
                axis,
                value: u8::from(buttons[offset + 1]) as f64 - u8::from(buttons[offset]) as f64,
            });
        })
}

fn arrow(data: &str) -> Element {
    path()
        .data(Geometry::parse(data).ok())
        .stroke("#EDF4F2")
        .stroke_thickness(2.5)
        .width(20.0)
        .height(20.0)
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Center)
        .is_hit_test_visible(false)
        .into()
}

pub fn controls(state: UiState, dispatch: Dispatch) -> Element {
    let held = signal([false; 8]);
    let place = dispatch.clone();
    let remove = dispatch.clone();
    let rotate = dispatch.clone();
    let exit = dispatch.clone();
    grid()
        .name("TouchControls")
        .is_visible(move || {
            let layout = state.layout.get();
            (layout.touch || layout.compact) && !layout.keyboard && !state.modal()
        })
        .children((
            border()
                .name("MovementPad")
                .horizontal_alignment(H::Left)
                .vertical_alignment(V::Bottom)
                .margin(move || (12.0, 0.0, 0.0, state.layout.get().controls_bottom()))
                .padding(5.0)
                .background("#B811222D")
                .corner_radius(16.0)
                .child(
                    grid()
                        .column_definitions("Auto,Auto,Auto")
                        .row_definitions("Auto,Auto")
                        .children((
                            held_button(
                                "TouchDescend",
                                "Down",
                                "Descend",
                                1,
                                false,
                                held,
                                dispatch.clone(),
                            ),
                            held_button(
                                "TouchForward",
                                "",
                                "Move forward or accelerate",
                                2,
                                true,
                                held,
                                dispatch.clone(),
                            )
                            .grid_column(1)
                            .child(arrow("M 4 13 L 10 7 L 16 13")),
                            held_button(
                                "TouchAscend",
                                "Rise",
                                "Jump or ascend",
                                1,
                                true,
                                held,
                                dispatch.clone(),
                            )
                            .grid_column(2),
                            held_button(
                                "TouchLeft",
                                "",
                                "Move left or steer left",
                                0,
                                false,
                                held,
                                dispatch.clone(),
                            )
                            .grid_row(1)
                            .child(arrow("M 13 4 L 7 10 L 13 16")),
                            held_button(
                                "TouchBack",
                                "",
                                "Move backward or reverse",
                                2,
                                false,
                                held,
                                dispatch.clone(),
                            )
                            .grid_row(1)
                            .grid_column(1)
                            .child(arrow("M 4 7 L 10 13 L 16 7")),
                            held_button(
                                "TouchRight",
                                "",
                                "Move right or steer right",
                                0,
                                true,
                                held,
                                dispatch.clone(),
                            )
                            .grid_row(1)
                            .grid_column(2)
                            .child(arrow("M 7 4 L 13 10 L 7 16")),
                        )),
                ),
            border()
                .name("TouchActions")
                .horizontal_alignment(H::Right)
                .vertical_alignment(V::Bottom)
                .margin(move || (0.0, 0.0, 12.0, state.layout.get().controls_bottom()))
                .padding(5.0)
                .background("#B811222D")
                .corner_radius(16.0)
                .child(
                    grid().children((
                        grid()
                            .is_visible(move || state.mode.get() != "DRIVE")
                            .column_definitions("Auto,Auto")
                            .row_definitions("Auto,Auto")
                            .children((
                                button()
                                    .name("TouchPlace")
                                    .grid_column_span(2)
                                    .class("primary")
                                    .content(move || {
                                        let play = state.gameplay.get();
                                        if play.expedition {
                                            format!("Place · {}", play.selected_cost)
                                        } else {
                                            "Place brick".into()
                                        }
                                    })
                                    .font_size(12.0)
                                    .is_enabled(move || state.gameplay.get().selected_affordable)
                                    .height(48.0)
                                    .margin(2.0)
                                    .padding((8.0, 0.0))
                                    .focusable(false)
                                    .tool_tip_tip("Place a brick at the target")
                                    .on_click(move |_| place(UiAction::Place)),
                                button()
                                    .name("TouchRotate")
                                    .grid_row(1)
                                    .content("Rotate")
                                    .width(52.0)
                                    .height(48.0)
                                    .margin(2.0)
                                    .padding(0.0)
                                    .font_size(11.0)
                                    .focusable(false)
                                    .on_click(move |_| rotate(UiAction::Rotate)),
                                button()
                                    .name("TouchRemove")
                                    .grid_row(1)
                                    .grid_column(1)
                                    .content("Remove")
                                    .width(52.0)
                                    .height(48.0)
                                    .margin(2.0)
                                    .padding(0.0)
                                    .font_size(11.0)
                                    .focusable(false)
                                    .on_click(move |_| remove(UiAction::Remove)),
                            )),
                        grid()
                            .is_visible(move || state.mode.get() == "DRIVE")
                            .row_definitions("Auto,Auto")
                            .children((
                                held_button(
                                    "TouchBrake",
                                    "Brake",
                                    "Hold the vehicle brake",
                                    3,
                                    true,
                                    held,
                                    dispatch,
                                )
                                .width(108.0),
                                button()
                                    .name("TouchExit")
                                    .grid_row(1)
                                    .content("Park & exit")
                                    .height(48.0)
                                    .width(108.0)
                                    .margin(2.0)
                                    .padding(0.0)
                                    .focusable(false)
                                    .on_click(move |_| exit(UiAction::ExitVehicle)),
                            )),
                    )),
                ),
        ))
        .into()
}
