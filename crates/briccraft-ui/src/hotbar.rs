use crate::{controls, preview, theme, ColorSwatch, Dispatch, PartCard, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextTrimming};
use ferroui_view::prelude::*;
use std::rc::Rc;

pub fn hotbar(
    parts: Rc<Vec<PartCard>>,
    colors: Rc<Vec<ColorSwatch>>,
    state: UiState,
    dispatch: Dispatch,
) -> Element {
    let rotate = dispatch.clone();
    let place = dispatch.clone();
    border()
        .name("Hotbar")
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Bottom)
        .margin(move || (12.0, 0.0, 12.0, state.layout.get().hotbar_bottom()))
        .is_visible(move || {
            !state.modal() && state.mode.get() != "DRIVE" && !state.layout.get().keyboard
        })
        .background(theme::SURFACE)
        .border_brush(theme::EDGE)
        .border_thickness(1.0)
        .corner_radius(14.0)
        .padding(move || {
            if state.compact.get() {
                (10.0, 8.0)
            } else {
                (12.0, 10.0)
            }
        })
        .effect(theme::shadow())
        .child(
            stack_panel().spacing(7.0).children((
                grid()
                    .column_definitions("*,Auto")
                    .is_visible(move || !state.layout.get().landscape)
                    .children((
                        stack_panel()
                            .orientation(Orientation::Horizontal)
                            .spacing(9.0)
                            .vertical_alignment(V::Center)
                            .children((
                                text_block()
                                    .text(state.selected_name)
                                    .max_width(
                                        move || if state.compact.get() { 234.0 } else { 350.0 },
                                    )
                                    .max_lines(1)
                                    .text_trimming(<dyn TextTrimming>::character_ellipsis())
                                    .font_size(12.0)
                                    .font_weight(FontWeight::SemiBold),
                                text_block()
                                    .text(state.selected_size)
                                    .font_size(10.0)
                                    .foreground(theme::MUTED)
                                    .is_visible(move || !state.compact.get()),
                            )),
                        stack_panel()
                            .grid_column(1)
                            .is_visible(move || !state.compact.get())
                            .orientation(Orientation::Horizontal)
                            .spacing(5.0)
                            .children((
                                button()
                                    .class("quiet")
                                    .content("Rotate  R")
                                    .height(32.0)
                                    .min_height(32.0)
                                    .padding((6.0, 2.0))
                                    .tool_tip_tip("Rotate selected part by 90° · R")
                                    .on_click(move |_| rotate(UiAction::Rotate)),
                                button()
                                    .class("primary")
                                    .content("Place")
                                    .height(32.0)
                                    .min_height(32.0)
                                    .padding((8.0, 2.0))
                                    .tool_tip_tip("Place on the targeted stud · left click")
                                    .on_click(move |_| place(UiAction::Place)),
                            )),
                    )),
                rerender(move || {
                    let selected = state.selected_part.get();
                    let color = colors
                        .iter()
                        .find(|c| c.id == state.selected_color.get())
                        .map_or(theme::ACCENT, |c| c.hex.as_str());
                    let count = state.layout.get().hotbar_slots();
                    let tiles: Vec<Element> = crate::HOTBAR_PARTS
                        .iter()
                        .take(count)
                        .filter_map(|id| parts.iter().find(|part| part.id == *id))
                        .enumerate()
                        .map(|(slot, part)| {
                            let id = part.id;
                            let dispatch = dispatch.clone();
                            button()
                                .name(format!("HotbarPart{}", part.id))
                                .width(if state.compact.get() { 54.0 } else { 67.0 })
                                .height(if state.compact.get() { 52.0 } else { 60.0 })
                                .padding(0.0)
                                .background(if part.id == selected {
                                    "#533A29"
                                } else {
                                    "#1B2B34"
                                })
                                .border_brush(if part.id == selected {
                                    theme::ACCENT
                                } else {
                                    "#324650"
                                })
                                .border_thickness(if part.id == selected { 2.0 } else { 1.0 })
                                .tool_tip_tip(format!("{} · {}", part.name, slot + 1))
                                .on_click(move |_| dispatch(UiAction::SelectPart(id)))
                                .child(
                                    grid().children((
                                        preview::brick(
                                            part,
                                            color,
                                            if state.compact.get() { 0.75 } else { 0.88 },
                                        ),
                                        text_block()
                                            .text((slot + 1).to_string())
                                            .font_size(9.0)
                                            .foreground(theme::MUTED)
                                            .horizontal_alignment(H::Left)
                                            .vertical_alignment(V::Top)
                                            .margin((6.0, 4.0, 0.0, 0.0)),
                                        border()
                                            .height(3.0)
                                            .width(16.0)
                                            .corner_radius(2.0)
                                            .background(theme::ACCENT)
                                            .horizontal_alignment(H::Center)
                                            .vertical_alignment(V::Bottom)
                                            .margin((0.0, 0.0, 0.0, 4.0))
                                            .is_visible(part.id == selected),
                                    )),
                                )
                                .into()
                        })
                        .collect();
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(6.0)
                        .children(tiles)
                        .into_element()
                }),
                stack_panel()
                    .orientation(Orientation::Horizontal)
                    .spacing(7.0)
                    .horizontal_alignment(H::Center)
                    .is_visible(move || !state.compact.get())
                    .children((
                        controls::keycap("LMB"),
                        controls::label("PLACE"),
                        controls::keycap("RMB"),
                        controls::label("REMOVE"),
                        controls::keycap("DRAG"),
                        controls::label("ORBIT"),
                        controls::keycap("B"),
                        controls::label("LIBRARY"),
                    )),
            )),
        )
        .into()
}
