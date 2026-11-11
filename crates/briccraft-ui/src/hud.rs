use crate::{controls, preview, theme, Dispatch, PartCard, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_view::prelude::*;

pub fn header(state: UiState, dispatch: Dispatch) -> Element {
    let explore = dispatch.clone();
    let save = dispatch;
    border()
        .name("Header")
        .vertical_alignment(V::Top)
        .margin((20.0, 18.0, 20.0, 0.0))
        .background(theme::SURFACE)
        .border_brush(theme::EDGE)
        .border_thickness(1.0)
        .corner_radius(14.0)
        .padding((14.0, 10.0))
        .effect(theme::shadow())
        .child(
            grid().column_definitions("Auto,*,Auto").children((
                stack_panel()
                    .orientation(Orientation::Horizontal)
                    .spacing(10.0)
                    .vertical_alignment(V::Center)
                    .children((
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
                        stack_panel()
                            .spacing(0.0)
                            .vertical_alignment(V::Center)
                            .children((
                                text_block()
                                    .text("BricCraft")
                                    .font_size(24.0)
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
                    .horizontal_alignment(H::Center)
                    .vertical_alignment(V::Center)
                    .orientation(Orientation::Horizontal)
                    .spacing(6.0)
                    .is_visible(move || !state.compact.get())
                    .children((
                        border()
                            .background("#293C35")
                            .corner_radius(6.0)
                            .padding((10.0, 7.0))
                            .child(
                                text_block()
                                    .text(state.mode)
                                    .font_size(10.0)
                                    .foreground("#AFE5BC")
                                    .font_weight(FontWeight::SemiBold),
                            ),
                        button()
                            .class("quiet")
                            .content("Explore  ↗")
                            .tool_tip_tip(
                                "Switch between builder orbit and first-person exploration · Tab",
                            )
                            .on_click(move |_| explore(UiAction::ToggleExplore)),
                        button()
                            .class("quiet")
                            .content("Blueprints")
                            .tool_tip_tip("Build a vehicle or landmark from a reusable blueprint")
                            .on_click(move |_| {
                                let open = !state.blueprints_open.get_untracked();
                                state.close_panels();
                                state.blueprints_open.set(open);
                            }),
                    )),
                stack_panel()
                    .grid_column(2)
                    .orientation(Orientation::Horizontal)
                    .spacing(6.0)
                    .vertical_alignment(V::Center)
                    .children((
                        button()
                            .class("quiet")
                            .content("Save")
                            .tool_tip_tip("Save world in this browser · Ctrl+S")
                            .is_visible(move || !state.compact.get())
                            .on_click(move |_| save(UiAction::Save)),
                        button()
                            .class("quiet")
                            .content("☷")
                            .tool_tip_tip("World settings, save files and quality")
                            .width(36.0)
                            .padding(0.0)
                            .font_size(18.0)
                            .on_click(move |_| {
                                let open = !state.settings_open.get_untracked();
                                state.close_panels();
                                state.settings_open.set(open);
                            }),
                        button()
                            .class("primary")
                            .content(move || {
                                if state.compact.get() {
                                    "Bricks"
                                } else {
                                    "Brick library  B"
                                }
                            })
                            .tool_tip_tip("Browse every building part · B")
                            .on_click(move |_| {
                                let open = !state.catalog_open.get_untracked();
                                state.close_panels();
                                state.catalog_open.set(open);
                            }),
                    )),
            )),
        )
        .into()
}

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
                controls::label("YOUR CREATIVE WORLD"),
                text_block()
                    .text(state.world_name)
                    .font_size(19.0)
                    .font_weight(FontWeight::SemiBold),
                text_block()
                    .text(state.stats)
                    .font_size(11.0)
                    .foreground(theme::MUTED),
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
                    .content("↺  Reset camera")
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
        .is_visible(move || {
            !state.catalog_open.get()
                && !state.blueprints_open.get()
                && !state.settings_open.get()
                && !state.help_open.get()
        })
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
        .vertical_alignment(V::Bottom)
        .margin((24.0, 0.0, 24.0, 14.0))
        .column_definitions("Auto,*,Auto")
        .children((
            stack_panel()
                .orientation(Orientation::Horizontal)
                .spacing(5.0)
                .children((
                    button()
                        .class("quiet")
                        .content("?")
                        .width(32.0)
                        .height(30.0)
                        .min_height(30.0)
                        .padding(0.0)
                        .tool_tip_tip("Controls and building guide · H")
                        .on_click(move |_| {
                            let open = !state.help_open.get_untracked();
                            state.close_panels();
                            state.help_open.set(open);
                        }),
                    button()
                        .class("quiet")
                        .content("↶")
                        .width(32.0)
                        .height(30.0)
                        .min_height(30.0)
                        .padding(0.0)
                        .tool_tip_tip("Undo · Ctrl+Z")
                        .on_click(move |_| undo(UiAction::Undo)),
                    button()
                        .class("quiet")
                        .content("↷")
                        .width(32.0)
                        .height(30.0)
                        .min_height(30.0)
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
                        .max_width(680.0),
                ),
            border()
                .grid_column(2)
                .vertical_alignment(V::Center)
                .padding((10.0, 5.0))
                .background("#B0101C24")
                .corner_radius(5.0)
                .child(
                    text_block()
                        .text(move || format!("SKIA  ·  {:.0} FPS", state.fps.get()))
                        .font_size(10.0)
                        .foreground("#C5D6DC"),
                ),
        ))
        .into()
}
