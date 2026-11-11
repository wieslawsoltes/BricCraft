use crate::{controls, preview, theme, ColorSwatch, Dispatch, PartCard, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;
use std::rc::Rc;

pub fn catalog(
    parts: Rc<Vec<PartCard>>,
    colors: Rc<Vec<ColorSwatch>>,
    state: UiState,
    dispatch: Dispatch,
) -> Element {
    let search = signal(String::new());
    let category = signal(String::from("All"));
    let mut categories = vec![String::from("All")];
    for part in parts.iter() {
        if !categories.contains(&part.category) {
            categories.push(part.category.clone());
        }
    }
    let color_dispatch = dispatch.clone();
    let color_data = colors.clone();
    let part_count = parts.len();
    let content = grid()
        .row_definitions("Auto,Auto,*,Auto")
        .children((
            text_box()
                .name("PartSearch")
                .placeholder_text("Search bricks, axles, windows…")
                .text(search.two_way())
                .margin((0.0, 0.0, 0.0, 12.0)),
            rerender(move || {
                let selected = category.get();
                let choices: Vec<Element> = categories
                    .iter()
                    .map(|name| {
                        let target = name.clone();
                        button()
                            .content(name.clone())
                            .font_size(10.0)
                            .padding((9.0, 5.0))
                            .min_height(29.0)
                            .margin((0.0, 0.0, 5.0, 5.0))
                            .corner_radius(6.0)
                            .background(if *name == selected {
                                "#533A29"
                            } else {
                                theme::INSET
                            })
                            .border_brush(if *name == selected {
                                theme::ACCENT
                            } else {
                                "Transparent"
                            })
                            .on_click(move |_| category.set(target.clone()))
                            .into()
                    })
                    .collect();
                if state.compact.get() {
                    scroll_viewer()
                        .grid_row(1)
                        .height(44.0)
                        .margin((0.0, 0.0, 0.0, 8.0))
                        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Auto)
                        .vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                        .child(
                            stack_panel()
                                .orientation(Orientation::Horizontal)
                                .children(choices),
                        )
                        .into_element()
                } else {
                    wrap_panel()
                        .grid_row(1)
                        .margin((0.0, 0.0, 0.0, 10.0))
                        .children(choices)
                        .into_element()
                }
            }),
            scroll_viewer()
                .grid_row(2)
                .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                .child(rerender(move || {
                    let query = search.get().trim().to_lowercase();
                    let group = category.get();
                    let selected = state.selected_part.get();
                    let color = color_data
                        .iter()
                        .find(|c| c.id == state.selected_color.get())
                        .map_or(theme::ACCENT, |c| c.hex.as_str());
                    let items: Vec<Element> = parts
                        .iter()
                        .filter(|part| {
                            (group == "All" || part.category == group)
                                && (query.is_empty()
                                    || part.name.to_lowercase().contains(&query)
                                    || part.category.to_lowercase().contains(&query)
                                    || part.id.to_string() == query)
                        })
                        .map(|part| {
                            part_card(
                                part,
                                color,
                                part.id == selected,
                                dispatch.clone(),
                                state.compact.get(),
                            )
                        })
                        .collect();
                    if items.is_empty() {
                        stack_panel()
                            .spacing(9.0)
                            .margin((6.0, 20.0))
                            .children((
                                text_block().text("No matching parts").font_size(16.0),
                                controls::caption("Try a shape, a category, or a shorter search."),
                            ))
                            .into_element()
                    } else {
                        // The vertical scrollbar consumes part of the panel's
                        // width. Star columns divide the actual viewport, so a
                        // scrollbar can never force every card onto its own row.
                        let rows = vec!["Auto"; items.len().div_ceil(2)].join(",");
                        let cards: Vec<Element> = items
                            .into_iter()
                            .enumerate()
                            .map(|(index, card)| {
                                border()
                                    .grid_row((index / 2) as i32)
                                    .grid_column((index % 2) as i32)
                                    .margin(if index % 2 == 0 {
                                        (0.0, 0.0, 4.0, 8.0)
                                    } else {
                                        (4.0, 0.0, 0.0, 8.0)
                                    })
                                    .child(card)
                                    .into()
                            })
                            .collect();
                        grid()
                            .column_definitions("*,*")
                            .row_definitions(&rows)
                            .children(cards)
                            .into_element()
                    }
                })),
            stack_panel()
                .grid_row(3)
                .spacing(10.0)
                .margin((0.0, 14.0, 0.0, 0.0))
                .children((
                    grid().column_definitions("*,Auto").children((
                        controls::label("MOLDED COLORS"),
                        text_block()
                            .grid_column(1)
                            .text(format!("{part_count} parts"))
                            .font_size(10.0)
                            .foreground(theme::MUTED),
                    )),
                    rerender(move || {
                        let selected = state.selected_color.get();
                        let swatches: Vec<Element> = colors
                            .iter()
                            .map(|color| {
                                let id = color.id;
                                let dispatch = color_dispatch.clone();
                                button()
                                    .name(format!("Color{}", color.id))
                                    .width(25.0)
                                    .height(25.0)
                                    .min_width(0.0)
                                    .min_height(0.0)
                                    .margin((0.0, 0.0, 6.0, 6.0))
                                    .padding(0.0)
                                    .corner_radius(5.0)
                                    .background(color.hex.as_str())
                                    .border_brush(if selected == id {
                                        "White"
                                    } else {
                                        "#334F616A"
                                    })
                                    .border_thickness(if selected == id { 2.0 } else { 1.0 })
                                    .tool_tip_tip(color.name.clone())
                                    .on_click(move |_| dispatch(UiAction::SelectColor(id)))
                                    .into()
                            })
                            .collect();
                        scroll_viewer()
                            .max_height(if state.compact.get() { 92.0 } else { 140.0 })
                            .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
                            .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
                            .child(wrap_panel().children(swatches))
                            .into_element()
                    }),
                )),
        ))
        .into();
    border()
        .is_visible(state.catalog_open)
        .child(controls::panel(
            "Brick library",
            "PICK. CONNECT. CREATE.",
            content,
            move || state.catalog_open.set(false),
            state,
        ))
        .into()
}

fn part_card(
    part: &PartCard,
    color: &str,
    selected: bool,
    dispatch: Dispatch,
    compact: bool,
) -> Element {
    let id = part.id;
    button()
        .name(format!("Part{}", part.id))
        .horizontal_alignment(H::Stretch)
        .height(112.0)
        .padding(7.0)
        .corner_radius(10.0)
        .horizontal_content_alignment(H::Stretch)
        .background(if selected { "#523D2F" } else { "#1C2D37" })
        .border_brush(if selected { theme::ACCENT } else { "#354954" })
        .border_thickness(if selected { 2.0 } else { 1.0 })
        .tool_tip_tip(format!(
            "{} · {} × {} studs · {} plates",
            part.name, part.size[0], part.size[2], part.size[1]
        ))
        .on_click(move |_| dispatch(UiAction::SelectPart(id)))
        .child(
            stack_panel().spacing(3.0).children((
                border()
                    .height(42.0)
                    .horizontal_alignment(H::Center)
                    .child(preview::brick(part, color, 0.9)),
                text_block()
                    .text(part.name.clone())
                    .font_size(11.0)
                    .font_weight(FontWeight::SemiBold)
                    .horizontal_alignment(H::Center)
                    .max_width(if compact { 102.0 } else { 130.0 })
                    .text_wrapping(TextWrapping::Wrap),
                text_block()
                    .text(format!(
                        "{} × {} · {}p",
                        part.size[0], part.size[2], part.size[1]
                    ))
                    .foreground(theme::MUTED)
                    .font_size(9.0)
                    .horizontal_alignment(H::Center)
                    .vertical_alignment(V::Bottom),
            )),
        )
        .into()
}
