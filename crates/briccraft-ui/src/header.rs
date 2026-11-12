//! Floating native navigation leaves the scene visible between focused control groups.
use crate::{
    controls,
    icons::{self, Icon},
    theme, Dispatch, UiAction, UiState,
};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::FontWeight;
use ferroui_controls::{Border, Button};
use ferroui_view::prelude::*;

fn toggle(state: UiState, panel: Signal<bool>) {
    let open = !panel.get_untracked();
    state.close_panels();
    panel.set(open);
}

fn pod(content: Element) -> El<Border> {
    border()
        .background(theme::HUD)
        .border_brush("#53647786")
        .border_thickness(1.0)
        .corner_radius(16.0)
        .padding(7.0)
        .effect(theme::shadow())
        .child(content)
}

fn navigation(
    name: &str,
    title: &str,
    hint: &str,
    icon: Icon,
    activate: impl Fn() + 'static,
) -> El<Button> {
    button()
        .name(name)
        .class("quiet")
        .height(44.0)
        .padding((9.0, 7.0))
        .automation_properties_name(Some(hint.to_string()))
        .tool_tip_tip(hint)
        .child(icons::label(icon, title, theme::TEXT))
        .on_click(move |_| activate())
}

fn phone_action(name: &str, title: &str, icon: Icon, activate: impl Fn() + 'static) -> El<Button> {
    button()
        .name(name)
        .height(48.0)
        .padding((2.0, 3.0))
        .margin((2.0, 0.0))
        .background(theme::HUD)
        .border_brush("#53647786")
        .automation_properties_name(Some(title.to_string()))
        .tool_tip_tip(title)
        .child(
            stack_panel()
                .spacing(0.0)
                .is_hit_test_visible(false)
                .children((
                    icons::glyph(icon, theme::TEXT),
                    text_block()
                        .text(title)
                        .font_size(10.0)
                        .horizontal_alignment(H::Center),
                )),
        )
        .on_click(move |_| activate())
}

fn brand(state: UiState) -> Element {
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(9.0)
        .vertical_alignment(V::Center)
        .margin((5.0, 0.0))
        .children((
            border()
                .width(36.0)
                .height(36.0)
                .corner_radius(10.0)
                .background(theme::ACCENT)
                .is_visible(move || !state.compact.get() || state.layout.get().landscape)
                .child(icons::glyph(Icon::Bricks, theme::ACCENT_DARK)),
            stack_panel()
                .spacing(0.0)
                .vertical_alignment(V::Center)
                .children((
                    text_block()
                        .text("BricCraft")
                        .font_size(move || if state.compact.get() { 18.0 } else { 21.0 })
                        .font_weight(FontWeight::Bold),
                    text_block()
                        .text(move || format!("{} MODE", state.mode.get()))
                        .font_size(8.0)
                        .foreground(theme::POSITIVE),
                )),
        ))
        .into()
}

fn desktop_navigation(state: UiState, dispatch: Dispatch) -> Element {
    let worlds = dispatch.clone();
    let places = dispatch.clone();
    let explore = dispatch;
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(2.0)
        .children((
            navigation(
                "OpenWorlds",
                "Worlds",
                "Your worlds · G",
                Icon::World,
                move || worlds(UiAction::ShowWorlds),
            ),
            navigation(
                "OpenPlaces",
                "Places",
                "Build a real place",
                Icon::Places,
                move || places(UiAction::ShowPlaces),
            ),
            navigation(
                "ExploreWorld",
                "Explore",
                "Cycle Build, Fly and Walk · Tab",
                Icon::Explore,
                move || explore(UiAction::ToggleExplore),
            ),
            navigation(
                "OpenPlans",
                "Plans",
                "Starter plans and your saved designs",
                Icon::Plans,
                move || toggle(state, state.blueprints_open),
            ),
        ))
        .into()
}

fn landscape_navigation(state: UiState, dispatch: Dispatch) -> Element {
    let worlds = dispatch.clone();
    let places = dispatch.clone();
    let explore = dispatch;
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(2.0)
        .children((
            controls::tool_button("LandscapeWorlds", "Your worlds", Icon::World, move || {
                worlds(UiAction::ShowWorlds)
            }),
            controls::tool_button(
                "LandscapePlaces",
                "Choose a real place",
                Icon::Places,
                move || places(UiAction::ShowPlaces),
            ),
            controls::tool_button(
                "LandscapeExplore",
                "Change camera mode",
                Icon::Explore,
                move || explore(UiAction::ToggleExplore),
            ),
            controls::tool_button(
                "LandscapePlans",
                "Construction plans",
                Icon::Plans,
                move || toggle(state, state.blueprints_open),
            ),
            controls::tool_button(
                "LandscapeGoals",
                "Play and objectives",
                Icon::Goals,
                move || crate::gameplay::toggle(state),
            ),
        ))
        .into()
}

fn phone_navigation(state: UiState, dispatch: Dispatch) -> Element {
    let worlds = dispatch.clone();
    let places = dispatch.clone();
    let explore = dispatch.clone();
    let drive = dispatch;
    grid()
        .name("CompactActions")
        .column_definitions("*,*,*,*,*")
        .children((
            phone_action("PhoneWorlds", "Worlds", Icon::World, move || {
                worlds(UiAction::ShowWorlds)
            }),
            phone_action("PhonePlaces", "Places", Icon::Places, move || {
                places(UiAction::ShowPlaces)
            })
            .grid_column(1),
            phone_action("PhoneExplore", "Explore", Icon::Explore, move || {
                explore(UiAction::ToggleExplore)
            })
            .grid_column(2),
            phone_action("PhonePlans", "Plans", Icon::Plans, move || {
                toggle(state, state.blueprints_open)
            })
            .grid_column(3),
            button()
                .name("PhoneDrive")
                .grid_column(4)
                .height(48.0)
                .padding((2.0, 3.0))
                .margin((2.0, 0.0))
                .background(theme::HUD)
                .border_brush("#53647786")
                .tool_tip_tip("Drive a construction, or park the active vehicle")
                .child(rerender(move || {
                    let driving = state.mode.get() == "DRIVE";
                    stack_panel()
                        .spacing(0.0)
                        .is_hit_test_visible(false)
                        .children((
                            icons::glyph(
                                if driving { Icon::Park } else { Icon::Drive },
                                theme::TEXT,
                            ),
                            text_block()
                                .text(if driving { "Park" } else { "Drive" })
                                .font_size(10.0)
                                .horizontal_alignment(H::Center),
                        ))
                        .into_element()
                }))
                .on_click(move |_| {
                    drive(if state.mode.get_untracked() == "DRIVE" {
                        UiAction::ExitVehicle
                    } else {
                        UiAction::ActivateVehicle("auto".into())
                    })
                }),
        ))
        .into()
}

pub fn view(state: UiState, dispatch: Dispatch) -> Element {
    let save = dispatch.clone();
    grid()
        .name("Header")
        .vertical_alignment(V::Top)
        .margin(move || {
            if state.compact.get() {
                (12.0, 8.0, 12.0, 0.0)
            } else {
                (20.0, 16.0, 20.0, 0.0)
            }
        })
        .is_visible(move || !state.layout.get().keyboard)
        .row_definitions("Auto,Auto")
        .children((
            grid().column_definitions("Auto,*,Auto").children((
                pod(brand(state)),
                pod(desktop_navigation(state, dispatch.clone()))
                    .grid_column(1)
                    .horizontal_alignment(H::Center)
                    .is_visible(move || !state.compact.get()),
                pod(landscape_navigation(state, dispatch.clone()))
                    .grid_column(1)
                    .horizontal_alignment(H::Center)
                    .is_visible(move || state.layout.get().landscape),
                pod(stack_panel()
                    .orientation(Orientation::Horizontal)
                    .spacing(4.0)
                    .children((
                        controls::tool_button(
                            "QuickSave",
                            "Save this world · Ctrl+S",
                            Icon::Save,
                            move || save(UiAction::Save),
                        )
                        .is_visible(move || state.layout.get().width > 1120.0),
                        controls::tool_button(
                            "OpenSettings",
                            "Game menu and display settings",
                            Icon::Settings,
                            move || toggle(state, state.settings_open),
                        ),
                        button()
                            .name("OpenBricks")
                            .class("primary")
                            .height(44.0)
                            .padding((12.0, 7.0))
                            .tool_tip_tip("Choose a brick and color · B")
                            .child(rerender(move || {
                                icons::label(
                                    Icon::Bricks,
                                    if state.compact.get() {
                                        "Bricks"
                                    } else {
                                        "Brick library"
                                    },
                                    theme::ACCENT_DARK,
                                )
                            }))
                            .on_click(move |_| toggle(state, state.catalog_open)),
                    ))
                    .into())
                .grid_column(2),
            )),
            border()
                .grid_row(1)
                .margin((0.0, 6.0, 0.0, 0.0))
                .is_visible(move || state.compact.get() && !state.layout.get().landscape)
                .child(phone_navigation(state, dispatch)),
        ))
        .into()
}
