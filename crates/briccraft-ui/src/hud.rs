use crate::{controls, theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextTrimming, TextWrapping};
use ferroui_view::prelude::*;

pub fn world_card(state: UiState, dispatch: Dispatch) -> Element {
    let reset = dispatch.clone();
    let vehicle = dispatch.clone();
    let exit = dispatch;
    border()
        .name("WorldCard")
        .horizontal_alignment(H::Left)
        .vertical_alignment(V::Top)
        .margin((24.0, 112.0, 0.0, 0.0))
        .width(242.0)
        .padding(16.0)
        .corner_radius(12.0)
        .background("#D5101E26")
        .border_brush("#4A60717B")
        .border_thickness(1.0)
        .is_visible(move || !state.compact.get())
        .effect(theme::shadow())
        .child(
            stack_panel().spacing(10.0).children((
                controls::label("YOUR WORLD"),
                text_block()
                    .text(state.world_name)
                    .font_size(19.0)
                    .text_wrapping(TextWrapping::Wrap)
                    .max_lines(2)
                    .text_trimming(<dyn TextTrimming>::character_ellipsis())
                    .font_weight(FontWeight::SemiBold),
                text_block()
                    .text(state.stats)
                    .font_size(11.0)
                    .text_wrapping(ferroui_base::media::TextWrapping::Wrap)
                    .foreground(theme::MUTED),
                text_block()
                    .text(state.coordinates)
                    .font_size(10.0)
                    .foreground(theme::MUTED),
                crate::gameplay::summary(state),
                border()
                    .height(1.0)
                    .background("#3B4D56")
                    .margin((0.0, 3.0)),
                text_block()
                    .text("Build it. Bring it to life.")
                    .font_size(13.0)
                    .font_weight(FontWeight::SemiBold),
                text_block()
                    .text(state.mechanics)
                    .font_size(11.0)
                    .foreground(theme::MUTED)
                    .text_wrapping(ferroui_base::media::TextWrapping::Wrap),
                stack_panel()
                    .orientation(Orientation::Horizontal)
                    .spacing(6.0)
                    .children((
                        button()
                            .content("Drive  F")
                            .tool_tip_tip(
                                "Activate the connected construction or enter a nearby vehicle",
                            )
                            .on_click(move |_| vehicle(UiAction::ActivateVehicle("auto".into()))),
                        button()
                            .class("quiet")
                            .content("Exit")
                            .tool_tip_tip("Leave the active vehicle")
                            .on_click(move |_| exit(UiAction::ExitVehicle)),
                    )),
                button()
                    .class("quiet")
                    .content("Reset camera")
                    .horizontal_alignment(H::Left)
                    .tool_tip_tip("Return to the island overview · Home")
                    .on_click(move |_| reset(UiAction::ResetCamera)),
            )),
        )
        .into()
}

pub fn crosshair(state: UiState) -> Element {
    grid()
        .name("Crosshair")
        .width(18.0)
        .height(18.0)
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Center)
        .is_hit_test_visible(false)
        .is_visible(move || !state.modal() && state.mode.get() != "BUILD")
        .children((
            border()
                .width(18.0)
                .height(2.0)
                .background("#E6FFFFFF")
                .vertical_alignment(V::Center),
            border()
                .width(2.0)
                .height(18.0)
                .background("#E6FFFFFF")
                .horizontal_alignment(H::Center),
            border()
                .width(4.0)
                .height(4.0)
                .background("#15212A")
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
                (24.0, 0.0, 24.0, 14.0)
            }
        })
        .column_definitions("Auto,*,Auto")
        .children((
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(5.0)
                .children((
                    button()
                        .class("quiet")
                        .content("Help")
                        .is_visible(move || !state.compact.get())
                        .width(44.0)
                        .height(44.0)
                        .padding(0.0)
                        .tool_tip_tip("Controls and building guide · H")
                        .on_click(move |_| {
                            let open = !state.help_open.get_untracked();
                            state.close_panels();
                            state.help_open.set(open);
                        }),
                    button()
                        .name("OpenGameplay")
                        .class("quiet")
                        .content(move || {
                            if state.gameplay.get().expedition {
                                "Goals"
                            } else {
                                "Play"
                            }
                        })
                        .width(48.0)
                        .height(44.0)
                        .padding(0.0)
                        .tool_tip_tip("Gameplay and objectives · O")
                        .on_click(move |_| crate::gameplay::toggle(state)),
                    button()
                        .class("quiet")
                        .content("Undo")
                        .width(48.0)
                        .height(44.0)
                        .padding(0.0)
                        .tool_tip_tip("Undo · Ctrl+Z")
                        .on_click(move |_| undo(UiAction::Undo)),
                    button()
                        .class("quiet")
                        .content("Redo")
                        .width(48.0)
                        .height(44.0)
                        .padding(0.0)
                        .tool_tip_tip("Redo · Ctrl+Y")
                        .on_click(move |_| redo(UiAction::Redo)),
                )),
            border()
                .grid_column(1)
                .horizontal_alignment(H::Center)
                .vertical_alignment(V::Center)
                .padding((12.0, 5.0))
                .corner_radius(5.0)
                .background("#B0101C24")
                .is_visible(move || !state.compact.get())
                .child(
                    text_block()
                        .text(state.status)
                        .foreground("#E0E8EA")
                        .font_size(11.0)
                        .max_width(680.0)
                        .max_lines(1)
                        .text_trimming(<dyn TextTrimming>::character_ellipsis()),
                ),
            border()
                .grid_column(2)
                .vertical_alignment(V::Center)
                .padding((10.0, 5.0))
                .background("#B0101C24")
                .corner_radius(5.0)
                .child(
                    text_block()
                        .text(move || {
                            let fps = state.fps.get();
                            if fps < 1.0 {
                                format!("SKIA  ·  {fps:.2} FPS")
                            } else if fps < 10.0 {
                                format!("SKIA  ·  {fps:.1} FPS")
                            } else {
                                format!("SKIA  ·  {fps:.0} FPS")
                            }
                        })
                        .font_size(10.0)
                        .foreground("#C5D6DC"),
                ),
        ))
        .into()
}

/// Mobile commands retain their feedback even when the desktop world card is hidden.
pub fn activity(state: UiState) -> Element {
    button()
        .name("MobileStatus")
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Top)
        .margin(move || (12.0, state.layout.get().header_bottom() + 4.0, 12.0, 0.0))
        .max_width(move || (state.layout.get().width - 24.0).clamp(1.0, 520.0))
        .padding((10.0, 7.0))
        .horizontal_content_alignment(H::Stretch)
        .corner_radius(8.0)
        .background("#B0101E26")
        .is_visible(move || state.compact.get() && !state.modal() && !state.layout.get().keyboard)
        .tool_tip_tip("Open gameplay, resources and objectives")
        .on_click(move |_| crate::gameplay::toggle(state))
        .child(
            stack_panel().spacing(3.0).children((
                text_block()
                    .text(move || {
                        let play = state.gameplay.get();
                        if play.expedition {
                            format!("EXPEDITION · {} Studs · Goals", play.studs)
                        } else {
                            "CREATIVE · unlimited bricks · Play modes".into()
                        }
                    })
                    .font_size(10.0)
                    .font_weight(FontWeight::SemiBold)
                    .foreground("#C7E8D2"),
                text_block()
                    .text(move || format!("{} · {}", state.mode.get(), state.status.get()))
                    .font_size(11.0)
                    .foreground("#E0E8EA")
                    .text_wrapping(TextWrapping::Wrap)
                    .max_lines(1)
                    .text_trimming(<dyn TextTrimming>::character_ellipsis()),
            )),
        )
        .into()
}
