//! Objective cards reflect rewarded, persistent progress from the simulation.
use crate::{controls, theme, ObjectiveCard, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_view::prelude::*;

fn card(goal: ObjectiveCard) -> Element {
    let progress = goal.progress.min(goal.target);
    let percent = 100.0 * progress as f64 / goal.target.max(1) as f64;
    border()
        .name(format!("Objective{}", goal.id))
        .background(if goal.completed {
            "#20382F"
        } else {
            theme::INSET
        })
        .border_brush(if goal.completed {
            "#426953"
        } else {
            theme::EDGE
        })
        .border_thickness(1.0)
        .corner_radius(10.0)
        .padding(13.0)
        .child(
            stack_panel().spacing(9.0).children((
                grid().column_definitions("*,Auto").children((
                    text_block()
                        .text(goal.title)
                        .font_size(13.0)
                        .font_weight(FontWeight::SemiBold)
                        .text_wrapping(TextWrapping::Wrap)
                        .margin((0.0, 0.0, 8.0, 0.0)),
                    text_block()
                        .grid_column(1)
                        .text(if goal.completed {
                            "COMPLETE"
                        } else {
                            "IN PROGRESS"
                        })
                        .font_size(8.0)
                        .font_weight(FontWeight::SemiBold)
                        .vertical_alignment(V::Center)
                        .foreground(if goal.completed {
                            "#AFE5BC"
                        } else {
                            theme::MUTED
                        }),
                )),
                controls::caption(goal.detail),
                progress_bar()
                    .minimum(0.0)
                    .maximum(100.0)
                    .value(percent)
                    .height(5.0)
                    .foreground(if goal.completed {
                        "#8CC89F"
                    } else {
                        theme::ACCENT
                    }),
                grid().column_definitions("*,Auto").children((
                    text_block()
                        .text(format!("{progress} / {}", goal.target))
                        .font_size(11.0)
                        .foreground(theme::MUTED),
                    text_block()
                        .grid_column(1)
                        .horizontal_alignment(H::Right)
                        .text(format!(
                            "{} +{} Studs",
                            if goal.completed { "Earned" } else { "Reward" },
                            goal.reward
                        ))
                        .font_size(11.0)
                        .foreground(if goal.completed { "#AFE5BC" } else { "#F2BD7B" }),
                )),
            )),
        )
        .into()
}

pub fn view(state: UiState) -> Element {
    stack_panel().spacing(10.0).children((
        grid().column_definitions("*,Auto").children((
            controls::label("YOUR OBJECTIVES"),
            text_block().grid_column(1).text(move || {
                let play = state.gameplay.get();
                format!("{} / {} complete", play.completed(), play.objectives.len())
            }).font_size(10.0).foreground(theme::MUTED),
        )),
        controls::caption("Progress advances in Expedition. Each objective rewards Studs once, automatically; your progress stays with this world."),
        rerender(move || {
            let cards: Vec<Element> = state.gameplay.get().objectives.into_iter().map(card).collect();
            stack_panel().spacing(8.0).children(cards).into_element()
        }),
    )).into()
}
