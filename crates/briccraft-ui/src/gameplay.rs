//! Native ruleset selection, resource ledger, objectives and workshop guidance.
use crate::{controls, objectives, theme, workshop, Dispatch, UiAction, UiState};
use ferroui_base::layout::HorizontalAlignment as H;
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

pub fn toggle(state: UiState) {
    let open = !state.gameplay_open.get_untracked();
    state.close_panels();
    state.gameplay_open.set(open);
}

fn modes(state: UiState, dispatch: Dispatch) -> Element {
    let creative = dispatch.clone();
    grid()
        .column_definitions("*,*")
        .children((
            button()
                .name("CreativeMode")
                .padding(12.0)
                .margin((0.0, 0.0, 4.0, 0.0))
                .horizontal_content_alignment(H::Stretch)
                .is_enabled(move || state.mode.get() != "DRIVE")
                .background(move || {
                    if state.gameplay.get().expedition {
                        theme::INSET
                    } else {
                        "#294143"
                    }
                })
                .border_brush(move || {
                    if state.gameplay.get().expedition {
                        theme::EDGE
                    } else {
                        "#83B8B4"
                    }
                })
                .on_click(move |_| creative(UiAction::SetExpedition(false)))
                .child(
                    stack_panel().spacing(6.0).children((
                        text_block()
                            .text("Creative")
                            .font_size(15.0)
                            .font_weight(FontWeight::SemiBold),
                        controls::caption("Unlimited building. Your expedition progress is kept."),
                    )),
                ),
            button()
                .name("ExpeditionMode")
                .grid_column(1)
                .padding(12.0)
                .margin((4.0, 0.0, 0.0, 0.0))
                .horizontal_content_alignment(H::Stretch)
                .is_enabled(move || state.mode.get() != "DRIVE")
                .background(move || {
                    if state.gameplay.get().expedition {
                        theme::SELECTED
                    } else {
                        theme::INSET
                    }
                })
                .border_brush(move || {
                    if state.gameplay.get().expedition {
                        theme::ACCENT
                    } else {
                        theme::EDGE
                    }
                })
                .on_click(move |_| dispatch(UiAction::SetExpedition(true)))
                .child(
                    stack_panel().spacing(6.0).children((
                        text_block()
                            .text("Expedition")
                            .font_size(15.0)
                            .font_weight(FontWeight::SemiBold),
                        text_block()
                            .text(move || {
                                if state.gameplay.get().started {
                                    "Resume earning, building and exploring."
                                } else {
                                    "Start once with 1,200 Studs. Earn as you explore."
                                }
                            })
                            .font_size(12.0)
                            .foreground(theme::MUTED)
                            .text_wrapping(TextWrapping::Wrap),
                    )),
                ),
        ))
        .into()
}

fn ledger(state: UiState) -> Element {
    border().background("#173C3B").corner_radius(14.0).padding(16.0)
        .child(stack_panel().spacing(9.0).children((
            controls::label("YOUR STUD BALANCE"),
            text_block().name("StudBalance").text(move || {
                let play = state.gameplay.get();
                if play.started { play.studs.to_string() } else { "Ready to start".into() }
            }).font_size(28.0).font_weight(FontWeight::Bold).foreground(theme::POSITIVE),
            text_block().text(move || if state.gameplay.get().expedition {
                "Build and recycle with Studs. Objective rewards arrive automatically."
            } else {
                "Unlimited building. Expedition balance and goals are paused and preserved."
            }).font_size(12.0).foreground("#B7D3C4").text_wrapping(TextWrapping::Wrap),
            text_block().text(move || {
                let play = state.gameplay.get();
                format!("{} · {} Studs{}", state.selected_name.get(), play.selected_cost,
                    if play.expedition { " per part" } else { " in Expedition" })
            }).font_size(11.0).foreground("#D9E8EF").text_wrapping(TextWrapping::Wrap),
        ))).into()
}

fn selection(state: UiState) -> Element {
    border()
        .background(theme::INSET)
        .corner_radius(14.0)
        .padding(14.0)
        .child(
            stack_panel().spacing(9.0).children((
                controls::label("SELECTED PART"),
                text_block()
                    .text(state.selected_name)
                    .font_size(14.0)
                    .font_weight(FontWeight::SemiBold)
                    .text_wrapping(TextWrapping::Wrap),
                text_block()
                    .name("SelectedPartCost")
                    .font_size(12.0)
                    .foreground(move || {
                        if state.gameplay.get().selected_affordable {
                            "#AFE5BC"
                        } else {
                            "#FFBEA3"
                        }
                    })
                    .text_wrapping(TextWrapping::Wrap)
                    .text(move || {
                        let play = state.gameplay.get();
                        if !play.expedition {
                            format!(
                                "Unlimited in Creative · Expedition cost {} Studs",
                                play.selected_cost
                            )
                        } else if play.selected_affordable {
                            format!(
                                "{} Studs per part · within your balance",
                                play.selected_cost
                            )
                        } else {
                            format!(
                                "{} Studs per part · {} more needed",
                                play.selected_cost,
                                play.selected_cost.saturating_sub(play.studs)
                            )
                        }
                    }),
                button().content("Choose another part").on_click(move |_| {
                    state.close_panels();
                    state.catalog_open.set(true);
                }),
            )),
        )
        .into()
}

pub fn panel(state: UiState, dispatch: Dispatch) -> Element {
    let section = signal(0u8);
    let content = scroll_viewer()
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(14.0).children((
            modes(state, dispatch.clone()),
            controls::caption("Switching modes clears undo history; your world and expedition progress remain."),
            text_block().text("Park your vehicle to change play modes.").font_size(12.0)
                .foreground("#F2BD7B").is_visible(move || state.mode.get() == "DRIVE"),
            ledger(state),
            grid().column_definitions("*,*").children((
                button().name("ObjectivesTab").content("Objectives").margin((0.0, 0.0, 4.0, 0.0))
                    .background(move || if section.get() == 0 { theme::SELECTED } else { theme::INSET })
                    .border_brush(move || if section.get() == 0 { theme::ACCENT } else { theme::EDGE })
                    .on_click(move |_| section.set(0)),
                button().name("WorkshopTab").grid_column(1).content("Workshop").margin((4.0, 0.0, 0.0, 0.0))
                    .background(move || if section.get() == 1 { theme::SELECTED } else { theme::INSET })
                    .border_brush(move || if section.get() == 1 { theme::ACCENT } else { theme::EDGE })
                    .on_click(move |_| section.set(1)),
            )),
            border().is_visible(move || section.get() == 0).child(objectives::view(state)),
            stack_panel().spacing(12.0).is_visible(move || section.get() == 1).children((
                selection(state),
                workshop::view(state, dispatch),
                border().background("#263D49").corner_radius(14.0).padding(14.0)
                    .child(text_block().name("GameplayGuidance").text(move || state.gameplay.get().guidance)
                        .font_size(12.0).foreground("#D9E8EF").text_wrapping(TextWrapping::Wrap)),
            )),
        ))).into();
    border()
        .is_visible(state.gameplay_open)
        .child(controls::panel(
            "Play & objectives",
            "BUILD. DISCOVER. BRING IT TO LIFE.",
            content,
            move || state.gameplay_open.set(false),
            state,
        ))
        .into()
}
