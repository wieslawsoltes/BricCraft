//! Starter constructions and reusable player designs share one native library.
use crate::{blueprint_preview, controls, designs, theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

const STARTERS: [(&str, &str, &str, &str); 8] = [
    (
        "rover",
        "Trail rover",
        "Motor, steering and four wheels",
        "#E8AC51",
    ),
    (
        "plane",
        "Island hopper",
        "Powered flight, wings and propeller",
        "#DF765D",
    ),
    (
        "train",
        "Coastal express",
        "A powered locomotive for the coastal railway",
        "#63B8CA",
    ),
    (
        "boat",
        "Lagoon sailboat",
        "An editable hull and sail for water travel",
        "#DCE7EF",
    ),
    (
        "house",
        "Island cottage",
        "A home with windows and a pitched roof",
        "#D98B64",
    ),
    (
        "tower",
        "Lookout tower",
        "A landmark above the treetops",
        "#AEC5D5",
    ),
    (
        "bridge",
        "Studstone bridge",
        "Connect your world across a river",
        "#B8AB94",
    ),
    (
        "windmill",
        "Meadow windmill",
        "A distinctive mechanical landmark",
        "#A5C5B6",
    ),
];

fn starter_card(
    id: &'static str,
    name: &'static str,
    detail: &'static str,
    color: &'static str,
    state: UiState,
    dispatch: Dispatch,
) -> Element {
    button()
        .name(format!("Blueprint{id}"))
        .horizontal_content_alignment(H::Stretch)
        .padding(12.0)
        .corner_radius(14.0)
        .background(theme::INSET)
        .is_enabled(move || {
            state
                .gameplay
                .get()
                .blueprint(id)
                .is_some_and(|quote| quote.affordable)
        })
        .on_click(move |_| {
            dispatch(UiAction::SpawnBlueprint(id.into()));
            state.blueprints_open.set(false);
        })
        .child(
            grid().column_definitions("Auto,*").children((
                border()
                    .width(86.0)
                    .height(80.0)
                    .corner_radius(11.0)
                    .background("#142333")
                    .vertical_alignment(V::Center)
                    .child(
                        border()
                            .horizontal_alignment(H::Center)
                            .vertical_alignment(V::Center)
                            .child(blueprint_preview::view(id, color)),
                    ),
                stack_panel()
                    .grid_column(1)
                    .margin((13.0, 0.0, 0.0, 0.0))
                    .spacing(6.0)
                    .vertical_alignment(V::Center)
                    .children((
                        text_block()
                            .text(name)
                            .font_size(14.0)
                            .font_weight(FontWeight::SemiBold)
                            .text_wrapping(TextWrapping::Wrap),
                        text_block()
                            .text(detail)
                            .font_size(11.0)
                            .foreground(theme::MUTED)
                            .text_wrapping(TextWrapping::Wrap),
                        text_block()
                            .text(move || {
                                let play = state.gameplay.get();
                                match play.blueprint(id) {
                                    Some(_) if !play.expedition => "Place in your world".into(),
                                    Some(quote) if quote.affordable => {
                                        format!("Build · {} Studs", quote.studs)
                                    }
                                    Some(quote) => format!(
                                        "{} Studs · {} more needed",
                                        quote.studs,
                                        quote.studs.saturating_sub(play.studs)
                                    ),
                                    None => "Quote unavailable".into(),
                                }
                            })
                            .font_size(11.0)
                            .font_weight(FontWeight::SemiBold)
                            .text_wrapping(TextWrapping::Wrap)
                            .foreground(move || {
                                if state
                                    .gameplay
                                    .get()
                                    .blueprint(id)
                                    .is_some_and(|quote| quote.affordable)
                                {
                                    theme::POSITIVE
                                } else {
                                    "#FFBEA3"
                                }
                            }),
                    )),
            )),
        )
        .into()
}

fn starters(state: UiState, dispatch: Dispatch) -> Element {
    let cards: Vec<Element> = STARTERS
        .into_iter()
        .map(|(id, name, detail, color)| {
            starter_card(id, name, detail, color, state, dispatch.clone())
        })
        .collect();
    scroll_viewer()
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(12.0).children((
            controls::caption("Start with a complete construction, then make it yours. Every piece stays editable."),
            stack_panel().spacing(10.0).children(cards),
            border().background("#203B3C").corner_radius(12.0).padding(14.0)
                .child(stack_panel().spacing(7.0).children((
                    controls::label("BRING IT TO LIFE"),
                    controls::caption("Build on clear ground. The Workshop checks power, connections and running gear before you drive. Save your own construction in My designs."),
                ))),
        )))
        .into()
}

pub fn panel(state: UiState, dispatch: Dispatch) -> Element {
    let section = state.blueprint_section;
    let content = grid()
        .row_definitions("Auto,*")
        .children((
            grid()
                .column_definitions("*,*")
                .margin((0.0, 0.0, 0.0, 14.0))
                .children((
                    button()
                        .name("StarterPlansTab")
                        .content("Starter plans")
                        .margin((0.0, 0.0, 4.0, 0.0))
                        .background(move || {
                            if section.get() == 0 {
                                theme::SELECTED
                            } else {
                                theme::INSET
                            }
                        })
                        .border_brush(move || {
                            if section.get() == 0 {
                                theme::ACCENT
                            } else {
                                theme::EDGE
                            }
                        })
                        .on_click(move |_| section.set(0)),
                    button()
                        .name("MyDesignsTab")
                        .grid_column(1)
                        .margin((4.0, 0.0, 0.0, 0.0))
                        .content(move || {
                            let count = state.designs.get().designs.len();
                            if count == 0 {
                                "My designs".into()
                            } else {
                                format!("My designs · {count}")
                            }
                        })
                        .background(move || {
                            if section.get() == 1 {
                                theme::SELECTED
                            } else {
                                theme::INSET
                            }
                        })
                        .border_brush(move || {
                            if section.get() == 1 {
                                theme::ACCENT
                            } else {
                                theme::EDGE
                            }
                        })
                        .on_click(move |_| section.set(1)),
                )),
            border()
                .grid_row(1)
                .is_visible(move || section.get() == 0)
                .child(starters(state, dispatch.clone())),
            border()
                .grid_row(1)
                .is_visible(move || section.get() == 1)
                .child(designs::view(state, dispatch)),
        ))
        .into();
    border()
        .is_visible(state.blueprints_open)
        .child(controls::panel(
            "Construction plans",
            "FROM FIRST BRICK TO NEXT ADVENTURE",
            content,
            move || state.blueprints_open.set(false),
            state,
        ))
        .into()
}
