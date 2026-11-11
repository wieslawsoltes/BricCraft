//! Compact identity, contextual vehicle control and readable activity feedback.
use crate::{
    controls,
    icons::{self, Icon},
    theme, Dispatch, UiAction, UiState,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextTrimming, TextWrapping};
use ferroui_view::prelude::*;

pub fn world_card(state: UiState, dispatch: Dispatch) -> Element {
    let worlds = dispatch.clone();
    let reset = dispatch.clone();
    grid()
        .name("WorldCard")
        .is_visible(move || !state.compact.get() && !state.modal())
        .children((
            stack_panel()
                .horizontal_alignment(H::Left)
                .vertical_alignment(V::Top)
                .margin((24.0, 96.0, 0.0, 0.0))
                .width(264.0)
                .spacing(8.0)
                .children((
                    border()
                        .background(theme::HUD)
                        .corner_radius(15.0)
                        .border_brush("#53647786")
                        .border_thickness(1.0)
                        .padding(5.0)
                        .child(
                            grid().column_definitions("*,Auto").children((
                                button()
                                    .class("quiet")
                                    .padding((9.0, 7.0))
                                    .horizontal_content_alignment(H::Stretch)
                                    .tool_tip_tip("Open your world collection · G")
                                    .on_click(move |_| worlds(UiAction::ShowWorlds))
                                    .child(
                                        stack_panel().spacing(4.0).children((
                                            controls::label("YOUR WORLD"),
                                            text_block()
                                                .text(state.world_name)
                                                .font_size(15.0)
                                                .font_weight(FontWeight::SemiBold)
                                                .max_lines(2)
                                                .text_wrapping(TextWrapping::Wrap)
                                                .text_trimming(
                                                    <dyn TextTrimming>::character_ellipsis(),
                                                ),
                                        )),
                                    ),
                                controls::tool_button(
                                    "ResetOverview",
                                    "Reset the camera · Home",
                                    Icon::Home,
                                    move || reset(UiAction::ResetCamera),
                                )
                                .grid_column(1)
                                .vertical_alignment(V::Center),
                            )),
                        ),
                    button()
                        .name("GameplaySummary")
                        .background(theme::HUD)
                        .border_brush("#53647786")
                        .padding((12.0, 10.0))
                        .horizontal_content_alignment(H::Stretch)
                        .tool_tip_tip("Play modes, resources and objectives · O")
                        .on_click(move |_| crate::gameplay::toggle(state))
                        .child(
                            grid().column_definitions("Auto,*,Auto").children((
                                icons::glyph(Icon::Goals, theme::POSITIVE),
                                stack_panel()
                                    .grid_column(1)
                                    .margin((10.0, 0.0, 8.0, 0.0))
                                    .spacing(2.0)
                                    .children((
                                        text_block()
                                            .text(move || state.gameplay.get().mode_label())
                                            .font_size(12.0)
                                            .font_weight(FontWeight::SemiBold),
                                        text_block()
                                            .text(move || {
                                                let play = state.gameplay.get();
                                                if play.expedition {
                                                    format!("{} Studs", play.studs)
                                                } else {
                                                    "Unlimited possibilities".into()
                                                }
                                            })
                                            .font_size(10.0)
                                            .foreground(theme::MUTED),
                                    )),
                                text_block()
                                    .grid_column(2)
                                    .text(move || {
                                        let play = state.gameplay.get();
                                        if play.expedition {
                                            format!(
                                                "{}/{}",
                                                play.completed(),
                                                play.objectives.len()
                                            )
                                        } else {
                                            "PLAY".into()
                                        }
                                    })
                                    .font_size(11.0)
                                    .foreground(theme::POSITIVE)
                                    .vertical_alignment(V::Center),
                            )),
                        ),
                )),
            vehicle(state, dispatch),
        ))
        .into()
}

fn vehicle(state: UiState, dispatch: Dispatch) -> Element {
    button()
        .name("ContextVehicle")
        .horizontal_alignment(H::Right)
        .vertical_alignment(V::Top)
        .margin((0.0, 96.0, 24.0, 0.0))
        .width(240.0)
        .padding(12.0)
        .background(theme::HUD)
        .border_brush("#597F79")
        .is_visible(move || state.mode.get() == "DRIVE" || state.gameplay.get().workshop.ready)
        .horizontal_content_alignment(H::Stretch)
        .tool_tip_tip("Use the ready construction · F, or park before editing")
        .on_click(move |_| {
            dispatch(if state.mode.get_untracked() == "DRIVE" {
                UiAction::ExitVehicle
            } else {
                UiAction::ActivateVehicle("auto".into())
            })
        })
        .child(
            grid().column_definitions("Auto,*").children((
                rerender(move || {
                    icons::glyph(
                        if state.mode.get() == "DRIVE" {
                            Icon::Park
                        } else {
                            Icon::Drive
                        },
                        theme::POSITIVE,
                    )
                }),
                stack_panel()
                    .grid_column(1)
                    .margin((10.0, 0.0, 0.0, 0.0))
                    .spacing(4.0)
                    .children((
                        text_block()
                            .text(move || {
                                if state.mode.get() == "DRIVE" {
                                    "Park & edit"
                                } else {
                                    "Drive construction"
                                }
                            })
                            .font_size(13.0)
                            .font_weight(FontWeight::SemiBold),
                        text_block()
                            .text(move || {
                                if state.mode.get() == "DRIVE" {
                                    state.mechanics.get()
                                } else {
                                    "Ready for adventure · F".into()
                                }
                            })
                            .font_size(10.0)
                            .foreground(theme::MUTED)
                            .max_lines(2)
                            .text_wrapping(TextWrapping::Wrap)
                            .text_trimming(<dyn TextTrimming>::character_ellipsis()),
                    )),
            )),
        )
        .into()
}

pub fn crosshair(state: UiState) -> Element {
    grid()
        .name("Crosshair")
        .width(20.0)
        .height(20.0)
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Center)
        .is_hit_test_visible(false)
        .is_visible(move || !state.modal() && matches!(state.mode.get().as_str(), "WALK" | "FLY"))
        .children((
            ellipse()
                .width(16.0)
                .height(16.0)
                .stroke("#CCFFFFFF")
                .stroke_thickness(1.4)
                .horizontal_alignment(H::Center)
                .vertical_alignment(V::Center),
            border()
                .width(4.0)
                .height(4.0)
                .corner_radius(2.0)
                .background("#FFFFFF")
                .horizontal_alignment(H::Center)
                .vertical_alignment(V::Center),
        ))
        .into()
}

pub fn footer(state: UiState, dispatch: Dispatch) -> Element {
    let undo = dispatch.clone();
    let redo = dispatch;
    grid()
        .name("Footer")
        .vertical_alignment(V::Bottom)
        .is_visible(move || !state.modal() && !state.layout.get().keyboard)
        .margin(move || {
            if state.compact.get() {
                (12.0, 0.0, 12.0, 8.0)
            } else {
                (24.0, 0.0, 24.0, 12.0)
            }
        })
        .column_definitions("Auto,*,Auto")
        .children((
            border()
                .background(theme::HUD)
                .corner_radius(13.0)
                .padding(3.0)
                .child(
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(2.0)
                        .children((
                            controls::tool_button(
                                "OpenHelp",
                                "Controls and building guide · H",
                                Icon::Help,
                                move || {
                                    state.close_panels();
                                    state.help_open.set(true);
                                },
                            )
                            .is_visible(move || !state.compact.get()),
                            controls::tool_button(
                                "OpenGameplay",
                                "Play, resources and objectives · O",
                                Icon::Goals,
                                move || crate::gameplay::toggle(state),
                            ),
                            controls::tool_button(
                                "UndoBuild",
                                "Undo construction · Ctrl+Z",
                                Icon::Undo,
                                move || undo(UiAction::Undo),
                            ),
                            controls::tool_button(
                                "RedoBuild",
                                "Redo construction · Ctrl+Y",
                                Icon::Redo,
                                move || redo(UiAction::Redo),
                            ),
                        )),
                ),
            button()
                .name("ActivityDetails")
                .grid_column(1)
                .class("quiet")
                .horizontal_alignment(H::Center)
                .vertical_alignment(V::Center)
                .padding((12.0, 8.0))
                .margin((12.0, 0.0))
                .corner_radius(10.0)
                .background(theme::HUD)
                .max_width(move || (state.layout.get().width - 390.0).clamp(180.0, 640.0))
                .is_visible(move || !state.compact.get())
                .tool_tip_tip("Read the complete activity message and world details")
                .on_click(move |_| crate::world_details::toggle(state))
                .child(
                    text_block()
                        .text(state.status)
                        .foreground("#DCE9F3")
                        .font_size(11.0)
                        .max_lines(1)
                        .text_trimming(<dyn TextTrimming>::character_ellipsis()),
                ),
            border()
                .name("PerformanceOverlay")
                .grid_column(2)
                .vertical_alignment(V::Center)
                .padding((10.0, 7.0))
                .background(theme::HUD)
                .corner_radius(9.0)
                .is_visible(state.show_metrics)
                .child(
                    text_block()
                        .text(move || format!("{:.1} FPS", state.fps.get()))
                        .font_size(10.0)
                        .foreground(theme::MUTED),
                ),
        ))
        .into()
}

/// Phone HUD chips provide direct access to goals and the full latest message.
pub fn activity(state: UiState) -> Element {
    grid()
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Top)
        .margin(move || (12.0, state.layout.get().header_bottom() + 6.0, 12.0, 0.0))
        .width(move || (state.layout.get().width - 24.0).clamp(1.0, 600.0))
        .column_definitions("Auto,*")
        .is_visible(move || state.compact.get() && !state.modal() && !state.layout.get().keyboard)
        .children((
            button()
                .name("MobileGameplay")
                .height(48.0)
                .padding((9.0, 7.0))
                .background(theme::HUD)
                .border_brush("#53647786")
                .tool_tip_tip("Play modes, resources and objectives")
                .on_click(move |_| crate::gameplay::toggle(state))
                .child(
                    stack_panel().spacing(2.0).children((
                        text_block()
                            .text(move || state.gameplay.get().mode_label())
                            .font_size(10.0)
                            .foreground(theme::POSITIVE)
                            .font_weight(FontWeight::SemiBold),
                        text_block()
                            .text(move || {
                                let play = state.gameplay.get();
                                if play.expedition {
                                    format!("{} Studs", play.studs)
                                } else {
                                    "Play your way".into()
                                }
                            })
                            .font_size(10.0),
                    )),
                ),
            button()
                .name("MobileStatus")
                .grid_column(1)
                .margin((6.0, 0.0, 0.0, 0.0))
                .height(48.0)
                .padding((10.0, 7.0))
                .horizontal_content_alignment(H::Stretch)
                .background(theme::HUD)
                .border_brush("#53647786")
                .tool_tip_tip("Read the complete activity message")
                .on_click(move |_| crate::world_details::toggle(state))
                .child(
                    text_block()
                        .text(state.status)
                        .font_size(11.0)
                        .foreground("#DCE9F3")
                        .text_wrapping(TextWrapping::Wrap)
                        .max_lines(2)
                        .text_trimming(<dyn TextTrimming>::character_ellipsis()),
                ),
        ))
        .into()
}
