//! Display choices explain the trade-off before changing the world renderer.
use crate::{
    controls,
    icons::{self, Icon},
    theme, Dispatch, UiAction, UiState,
};
use ferroui_base::layout::VerticalAlignment as V;
use ferroui_base::media::TextWrapping;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

fn choices(
    prefix: &'static str,
    options: &'static [(u8, &'static str)],
    selected: Signal<u8>,
    action: fn(u8) -> UiAction,
    dispatch: Dispatch,
) -> Element {
    rerender(move || {
        let selected = selected.get();
        let buttons: Vec<Element> = options
            .iter()
            .enumerate()
            .map(|(index, &(id, title))| {
                let dispatch = dispatch.clone();
                button()
                    .name(format!("{prefix}{id}"))
                    .grid_column(index as i32)
                    .content(title)
                    .padding((3.0, 9.0))
                    .margin((2.0, 0.0))
                    .font_size(11.0)
                    .background(if id == selected {
                        theme::SELECTED
                    } else {
                        theme::INSET
                    })
                    .border_brush(if id == selected {
                        theme::ACCENT
                    } else {
                        theme::EDGE
                    })
                    .on_click(move |_| dispatch(action(id)))
                    .into()
            })
            .collect();
        grid()
            .column_definitions(&vec!["*"; options.len()].join(","))
            .children(buttons)
            .into_element()
    })
    .into()
}

fn section(title: &str, content: Element) -> Element {
    border()
        .background(theme::INSET)
        .corner_radius(14.0)
        .padding(14.0)
        .child(
            stack_panel()
                .spacing(12.0)
                .children((controls::label(title), content)),
        )
        .into()
}

fn display(state: UiState, dispatch: Dispatch) -> Element {
    section("DISPLAY", stack_panel().spacing(12.0).children((
        choices("ResolutionMode", &[(0, "Auto"), (1, "Quality"), (2, "Performance")],
            state.resolution_mode, UiAction::SetResolutionMode, dispatch.clone()),
        text_block().text(move || match state.resolution_mode.get() {
            1 => "Prioritize crisp detail and the selected antialiasing. Very large displays may use a reduced world resolution.",
            2 => "Prioritize responsive motion. Detail and resolution may reduce further on slower devices.",
            _ => "Keep the image clear. Adjust shading and antialiasing before reducing resolution.",
        }).font_size(12.0).foreground(theme::MUTED).text_wrapping(TextWrapping::Wrap),
        controls::label("EDGE SMOOTHING"),
        choices("Antialiasing", &[(1, "Off"), (2, "2×"), (4, "4×")],
            state.antialiasing, UiAction::SetAntialiasing, dispatch.clone()),
        controls::caption("More samples smooth fine edges and studs. Auto and Performance can reduce sampling to keep motion responsive."),
        controls::label("SURFACE DETAIL"),
        choices("DetailQuality", &[(0, "Fast"), (1, "Balanced"), (2, "Cinematic")],
            state.quality, UiAction::SetQuality, dispatch),
        controls::caption("Higher detail adds more surface and lighting work. Menus and text stay sharp in every mode."),
        border().background("#142D32").corner_radius(9.0).padding(10.0)
            .child(text_block().name("RenderStatus").text(state.render_status)
                .font_size(11.0).foreground(theme::POSITIVE).text_wrapping(TextWrapping::Wrap)),
        check_box().content("Performance overlay")
            .is_checked(move || Some(state.show_metrics.get()))
            .on_click(move |_| state.show_metrics.set(!state.show_metrics.get_untracked())),
    )).into())
}

fn environment(state: UiState, dispatch: Dispatch) -> Element {
    let daylight = dispatch.clone();
    let sound = dispatch.clone();
    let grid_action = dispatch.clone();
    let pause = dispatch;
    section(
        "WORLD & SOUND",
        stack_panel()
            .spacing(10.0)
            .children((
                grid().column_definitions("*,Auto").children((
                    text_block().text("Time of day").font_size(13.0),
                    text_block()
                        .grid_column(1)
                        .text("Night → Day")
                        .font_size(10.0)
                        .foreground(theme::MUTED)
                        .vertical_alignment(V::Center),
                )),
                slider()
                    .name("Daylight")
                    .minimum(0.0)
                    .maximum(1.0)
                    .value(state.daylight)
                    .on_value_changed(move |event| {
                        daylight(UiAction::SetDaylight(event.new_value()))
                    }),
                check_box()
                    .content("Sound effects")
                    .is_checked(move || Some(state.sound.get()))
                    .on_click(move |_| sound(UiAction::SetSound(!state.sound.get_untracked()))),
                check_box()
                    .content("Construction grid")
                    .is_checked(move || Some(state.grid.get()))
                    .on_click(move |_| grid_action(UiAction::SetGrid(!state.grid.get_untracked()))),
                check_box()
                    .content("Pause simulation")
                    .is_checked(move || Some(state.paused.get()))
                    .on_click(move |_| pause(UiAction::SetPaused(!state.paused.get_untracked()))),
            ))
            .into(),
    )
}

fn files(state: UiState, dispatch: Dispatch) -> Element {
    let save = dispatch.clone();
    let load = dispatch.clone();
    let export = dispatch.clone();
    let import = dispatch;
    section("SAVE & SHARE", stack_panel().spacing(10.0).children((
        grid().column_definitions("*,*").row_definitions("Auto,Auto").children((
            button().name("SaveWorld").class("primary").margin((0.0, 0.0, 4.0, 6.0))
                .child(icons::label(Icon::Save, "Save", theme::ACCENT_DARK))
                .on_click(move |_| save(UiAction::Save)),
            button().grid_column(1).margin((4.0, 0.0, 0.0, 6.0)).content("Restore save")
                .on_click(move |_| load(UiAction::Load)),
            button().grid_row(1).margin((0.0, 0.0, 4.0, 0.0))
                .child(icons::label(Icon::Export, "Export", theme::TEXT))
                .on_click(move |_| export(UiAction::Export)),
            button().grid_row(1).grid_column(1).margin((4.0, 0.0, 0.0, 0.0))
                .child(icons::label(Icon::Import, "Import", theme::TEXT))
                .on_click(move |_| import(UiAction::Import)),
        )),
        controls::caption("Your worlds stay in this browser. Export a world to keep a backup or move it to another device."),
        button().class("quiet").content("World details & latest action")
            .on_click(move |_| crate::world_details::toggle(state)),
    )).into())
}

fn new_island(state: UiState, dispatch: Dispatch) -> Element {
    let expanded = signal(false);
    let seed = signal(state.seed.get_untracked().to_string());
    stack_panel().spacing(10.0).children((
        button().content(move || if expanded.get() { "Hide island generator" } else { "Create a classic island" })
            .on_click(move |_| expanded.set(!expanded.get_untracked())),
        stack_panel().spacing(10.0).is_visible(expanded).children((
            text_box().name("WorldSeed").text(seed.two_way()).placeholder_text("World seed"),
            button().content("Generate island").on_click(move |_| {
                match seed.get_untracked().parse::<u64>() {
                    Ok(value) => dispatch(UiAction::NewWorld(value)),
                    Err(_) => state.status.set("Enter a whole number for the world seed.".into()),
                }
            }),
            controls::caption("The current world is saved first. Use Worlds for open terrain and your saved collection."),
        )),
    )).into()
}

pub fn panel(state: UiState, dispatch: Dispatch) -> Element {
    let worlds = dispatch.clone();
    let reset = dispatch.clone();
    let content = scroll_viewer()
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(
            stack_panel().spacing(14.0).children((
                grid().column_definitions("*,*").children((
                    button()
                        .margin((0.0, 0.0, 4.0, 0.0))
                        .content("Play & objectives")
                        .font_size(11.0)
                        .padding((6.0, 9.0))
                        .on_click(move |_| crate::gameplay::toggle(state)),
                    button()
                        .grid_column(1)
                        .margin((4.0, 0.0, 0.0, 0.0))
                        .content("Controls & help")
                        .font_size(11.0)
                        .padding((6.0, 9.0))
                        .on_click(move |_| {
                            state.close_panels();
                            state.help_open.set(true);
                        }),
                )),
                display(state, dispatch.clone()),
                environment(state, dispatch.clone()),
                files(state, dispatch.clone()),
                section(
                    "EXPLORE",
                    grid()
                        .column_definitions("*,*")
                        .row_definitions("Auto,Auto")
                        .children((
                            button()
                                .content("Your worlds")
                                .margin((0.0, 0.0, 4.0, 6.0))
                                .on_click(move |_| worlds(UiAction::ShowWorlds)),
                            button()
                                .content("Real places")
                                .grid_column(1)
                                .margin((4.0, 0.0, 0.0, 6.0))
                                .on_click(move |_| {
                                    state.close_panels();
                                    state.imports_open.set(true);
                                }),
                            button()
                                .content("Plans & designs")
                                .padding((6.0, 9.0))
                                .grid_row(1)
                                .margin((0.0, 0.0, 4.0, 0.0))
                                .on_click(move |_| {
                                    state.close_panels();
                                    state.blueprints_open.set(true);
                                }),
                            button()
                                .content("Reset camera")
                                .grid_row(1)
                                .grid_column(1)
                                .margin((4.0, 0.0, 0.0, 0.0))
                                .on_click(move |_| reset(UiAction::ResetCamera)),
                        ))
                        .into(),
                ),
                new_island(state, dispatch),
            )),
        )
        .into();
    border()
        .is_visible(state.settings_open)
        .child(controls::panel(
            "Game menu",
            "MAKE THE WORLD FEEL YOURS",
            content,
            move || state.settings_open.set(false),
            state,
        ))
        .into()
}
