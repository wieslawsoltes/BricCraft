//! A wide toolbar and a two-row phone toolbar share the same native commands.
use crate::{preview, theme, Dispatch, PartCard, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;

fn toggle(state: UiState, panel: Signal<bool>) {
    let open = !panel.get_untracked();
    state.close_panels();
    panel.set(open);
}

pub fn view(state: UiState, dispatch: Dispatch) -> Element {
    let worlds = dispatch.clone();
    let explore = dispatch.clone();
    let vehicle = dispatch.clone();
    let mobile_worlds = dispatch.clone();
    let mobile_explore = dispatch.clone();
    let mobile_vehicle = dispatch.clone();
    let save = dispatch;
    border()
        .name("Header")
        .vertical_alignment(V::Top)
        .margin(move || {
            if state.compact.get() {
                (12.0, 8.0, 12.0, 0.0)
            } else {
                (20.0, 18.0, 20.0, 0.0)
            }
        })
        .is_visible(move || !state.layout.get().keyboard)
        .background(theme::SURFACE)
        .border_brush(theme::EDGE)
        .border_thickness(1.0)
        .corner_radius(14.0)
        .padding(move || {
            if state.compact.get() {
                (10.0, 8.0)
            } else {
                (14.0, 10.0)
            }
        })
        .effect(theme::shadow())
        .child(
            grid().row_definitions("Auto,Auto").children((
                grid().column_definitions("Auto,*,Auto").children((
                    stack_panel()
                        .orientation(Orientation::Horizontal)
                        .spacing(10.0)
                        .vertical_alignment(V::Center)
                        .children((
                            border().is_visible(move || !state.compact.get()).child(
                                preview::brick(
                                    &PartCard {
                                        id: 0,
                                        name: "Brick".into(),
                                        category: String::new(),
                                        size: [2, 3, 2],
                                    },
                                    theme::ACCENT,
                                    0.8,
                                ),
                            ),
                            stack_panel()
                                .spacing(1.0)
                                .vertical_alignment(V::Center)
                                .children((
                                    text_block()
                                        .text("BricCraft")
                                        .font_size(
                                            move || if state.compact.get() { 20.0 } else { 24.0 },
                                        )
                                        .font_weight(FontWeight::Bold),
                                    text_block()
                                        .text("A WORLD OF POSSIBILITIES")
                                        .font_size(8.0)
                                        .foreground(theme::MUTED)
                                        .is_visible(move || !state.compact.get()),
                                )),
                        )),
                    stack_panel()
                        .grid_column(1)
                        .orientation(Orientation::Horizontal)
                        .spacing(4.0)
                        .horizontal_alignment(H::Center)
                        .vertical_alignment(V::Center)
                        .is_visible(move || !state.compact.get() || state.layout.get().landscape)
                        .children((
                            border()
                                .background("#293C35")
                                .corner_radius(6.0)
                                .padding((10.0, 7.0))
                                .is_visible(move || !state.compact.get())
                                .child(
                                    text_block()
                                        .text(state.mode)
                                        .font_size(10.0)
                                        .foreground("#AFE5BC")
                                        .font_weight(FontWeight::SemiBold),
                                ),
                            button()
                                .name("OpenWorlds")
                                .class("quiet")
                                .content("Worlds")
                                .padding((9.0, 8.0))
                                .on_click(move |_| worlds(UiAction::ShowWorlds)),
                            button()
                                .class("quiet")
                                .content("Real places")
                                .is_visible(move || !state.compact.get())
                                .padding((9.0, 8.0))
                                .on_click(move |_| toggle(state, state.imports_open)),
                            button()
                                .class("quiet")
                                .content("Explore")
                                .padding((9.0, 8.0))
                                .on_click(move |_| explore(UiAction::ToggleExplore)),
                            button()
                                .class("quiet")
                                .content(move || {
                                    if state.compact.get() {
                                        "Plans"
                                    } else {
                                        "Blueprints"
                                    }
                                })
                                .padding((9.0, 8.0))
                                .on_click(move |_| toggle(state, state.blueprints_open)),
                            button()
                                .class("quiet")
                                .is_visible(move || state.layout.get().landscape)
                                .content(move || {
                                    if state.mode.get() == "DRIVE" {
                                        "Park"
                                    } else {
                                        "Drive"
                                    }
                                })
                                .padding((9.0, 8.0))
                                .on_click(move |_| {
                                    vehicle(if state.mode.get_untracked() == "DRIVE" {
                                        UiAction::ExitVehicle
                                    } else {
                                        UiAction::ActivateVehicle("auto".into())
                                    })
                                }),
                        )),
                    stack_panel()
                        .grid_column(2)
                        .orientation(Orientation::Horizontal)
                        .spacing(4.0)
                        .vertical_alignment(V::Center)
                        .children((
                            button()
                                .class("quiet")
                                .content("Save")
                                .is_visible(move || state.layout.get().width > 1120.0)
                                .on_click(move |_| save(UiAction::Save)),
                            button()
                                .name("OpenSettings")
                                .class("quiet")
                                .content(move || {
                                    if state.compact.get() {
                                        "Menu"
                                    } else {
                                        "Settings"
                                    }
                                })
                                .padding((9.0, 8.0))
                                .on_click(move |_| toggle(state, state.settings_open)),
                            button()
                                .name("OpenBricks")
                                .class("primary")
                                .content(move || {
                                    if state.compact.get() {
                                        "Bricks"
                                    } else {
                                        "Brick library  B"
                                    }
                                })
                                .padding((10.0, 8.0))
                                .on_click(move |_| toggle(state, state.catalog_open)),
                        )),
                )),
                grid()
                    .name("CompactActions")
                    .grid_row(1)
                    .column_definitions("*,*,*,*")
                    .margin((0.0, 6.0, 0.0, 0.0))
                    .is_visible(move || state.compact.get() && !state.layout.get().landscape)
                    .children((
                        button()
                            .content("Worlds")
                            .font_size(11.0)
                            .height(44.0)
                            .padding((4.0, 0.0))
                            .margin((0.0, 0.0, 3.0, 0.0))
                            .on_click(move |_| mobile_worlds(UiAction::ShowWorlds)),
                        button()
                            .grid_column(1)
                            .content("Explore")
                            .font_size(11.0)
                            .height(44.0)
                            .padding((4.0, 0.0))
                            .margin((2.0, 0.0, 2.0, 0.0))
                            .on_click(move |_| mobile_explore(UiAction::ToggleExplore)),
                        button()
                            .grid_column(2)
                            .content("Plans")
                            .font_size(11.0)
                            .height(44.0)
                            .padding((4.0, 0.0))
                            .margin((2.0, 0.0, 2.0, 0.0))
                            .on_click(move |_| toggle(state, state.blueprints_open)),
                        button()
                            .grid_column(3)
                            .font_size(11.0)
                            .height(44.0)
                            .padding((4.0, 0.0))
                            .margin((3.0, 0.0, 0.0, 0.0))
                            .content(move || {
                                if state.mode.get() == "DRIVE" {
                                    "Park"
                                } else {
                                    "Drive"
                                }
                            })
                            .on_click(move |_| {
                                mobile_vehicle(if state.mode.get_untracked() == "DRIVE" {
                                    UiAction::ExitVehicle
                                } else {
                                    UiAction::ActivateVehicle("auto".into())
                                })
                            }),
                    )),
            )),
        )
        .into()
}
