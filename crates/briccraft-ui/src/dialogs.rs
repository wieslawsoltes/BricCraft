//! Focused panels; every command is routed to the same application action handler.
use crate::{controls, preview, theme, Dispatch, PartCard, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

pub fn blueprints(state: UiState, dispatch: Dispatch) -> Element {
    let mut cards = Vec::new();
    for (id, name, detail, color, shape) in [
        (
            "rover",
            "Trail rover",
            "Motor · four wheels · steering",
            "#EBAA46",
            "Wheel",
        ),
        (
            "plane",
            "Island hopper",
            "Propeller · wings · flight controls",
            "#D46045",
            "Wing",
        ),
        (
            "train",
            "Coastal express",
            "Powered locomotive · rail wheels",
            "#4CA1B8",
            "Wheel",
        ),
        (
            "boat",
            "Lagoon sailboat",
            "Hull · sail · water travel",
            "#EBE9DE",
            "Slope",
        ),
        (
            "house",
            "Island cottage",
            "Walls · windows · pitched roof",
            "#DC986B",
            "Window",
        ),
        (
            "tower",
            "Lookout tower",
            "A landmark above the treetops",
            "#BAC7CE",
            "Brick",
        ),
        (
            "bridge",
            "Studstone bridge",
            "Connect the two riverbanks",
            "#AA9B84",
            "Beam",
        ),
        (
            "windmill",
            "Meadow windmill",
            "A mechanical landmark for your world",
            "#B0D1BD",
            "Axle",
        ),
    ] {
        let dispatch = dispatch.clone();
        let preview_part = PartCard {
            id: 0,
            name: shape.into(),
            category: String::new(),
            size: [4, 3, 2],
        };
        cards.push(
            button()
                .name(format!("Blueprint{id}"))
                .horizontal_content_alignment(H::Stretch)
                .padding(11.0)
                .background("#1E303A")
                .corner_radius(10.0)
                .on_click(move |_| {
                    dispatch(UiAction::SpawnBlueprint(id.into()));
                    state.blueprints_open.set(false);
                })
                .child(
                    grid().column_definitions("Auto,*").children((
                        border()
                            .width(64.0)
                            .vertical_alignment(V::Center)
                            .child(preview::brick(&preview_part, color, 0.88)),
                        stack_panel()
                            .grid_column(1)
                            .spacing(5.0)
                            .vertical_alignment(V::Center)
                            .children((
                                text_block()
                                    .text(name)
                                    .font_size(14.0)
                                    .font_weight(FontWeight::SemiBold),
                                text_block()
                                    .text(detail)
                                    .font_size(10.0)
                                    .foreground(theme::MUTED)
                                    .text_wrapping(TextWrapping::Wrap),
                            )),
                    )),
                )
                .into_element(),
        );
    }
    let content = scroll_viewer().horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(9.0).children((
            controls::caption("Place a complete construction, then add your own bricks. Select a vehicle and press F to take control."),
            stack_panel().spacing(8.0).children(cards),
            border().height(1.0).background(theme::EDGE).margin((0.0, 8.0)),
            controls::label("MAKE YOUR CONSTRUCTION MOVE"),
            controls::caption("Connect mechanical parts to a chassis. Wheels and a motor make a car; add wings and a propeller for flight. Inspect the connection status before activating."),
        ))).into();
    border()
        .is_visible(state.blueprints_open)
        .child(controls::panel(
            "Blueprints",
            "BUILD SOMETHING BIGGER",
            content,
            move || state.blueprints_open.set(false),
            state,
        ))
        .into()
}

pub fn settings(state: UiState, dispatch: Dispatch) -> Element {
    let sound = dispatch.clone();
    let grid_action = dispatch.clone();
    let pause = dispatch.clone();
    let quality = dispatch.clone();
    let daylight = dispatch.clone();
    let regenerate = dispatch.clone();
    let seed_text = signal(state.seed.get_untracked().to_string());
    let content = scroll_viewer().horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(12.0).children((
            controls::label("WORLD FILE"),
            wrap_panel().children([
                controls::action_button("Save", "Save to this browser", UiAction::Save, dispatch.clone()),
                controls::action_button("Load", "Restore the last browser save", UiAction::Load, dispatch.clone()),
                controls::action_button("Export", "Download the world as JSON", UiAction::Export, dispatch.clone()),
                controls::action_button("Import", "Open an exported world file", UiAction::Import, dispatch),
            ]),
            controls::caption("Worlds stay in your browser. Export a file to keep a backup or move a world to another device."),
            stack_panel().spacing(12.0).children((
            separator(), controls::label("DAYLIGHT"),
            slider().name("Daylight").minimum(0.0).maximum(1.0).value(state.daylight)
                .on_value_changed(move |e| daylight(UiAction::SetDaylight(e.new_value()))),
            grid().column_definitions("*,*,*").children((
                text_block().text("NIGHT").font_size(9.0).foreground(theme::MUTED),
                text_block().grid_column(1).text("DAWN").font_size(9.0).foreground(theme::MUTED).horizontal_alignment(H::Center),
                text_block().grid_column(2).text("DAY").font_size(9.0).foreground(theme::MUTED).horizontal_alignment(H::Right),
            )),
            )),
            stack_panel().spacing(12.0).children((
            separator(), controls::label("RENDER QUALITY"),
            rerender(move || {
                let selected = state.quality.get();
                let buttons: Vec<Element> = [(0, "Fast"), (1, "Balanced"), (2, "Cinematic")].into_iter().map(|(id, title)| {
                    let dispatch = quality.clone();
                    button().content(title).padding((10.0, 7.0)).font_size(11.0)
                        .background(if id == selected { "#533A29" } else { theme::INSET })
                        .border_brush(if id == selected { theme::ACCENT } else { theme::EDGE })
                        .on_click(move |_| dispatch(UiAction::SetQuality(id))).into()
                }).collect();
                stack_panel().orientation(Orientation::Horizontal).spacing(5.0).children(buttons).into_element()
            }),
            controls::caption("The world adapts detail and resolution to keep movement responsive. Your choice sets the maximum detail."),
            check_box().content("Sound effects").is_checked(move || Some(state.sound.get()))
                .on_click(move |_| sound(UiAction::SetSound(!state.sound.get_untracked()))),
            check_box().content("Construction grid").is_checked(move || Some(state.grid.get()))
                .on_click(move |_| grid_action(UiAction::SetGrid(!state.grid.get_untracked()))),
            check_box().content("Pause simulation").is_checked(move || Some(state.paused.get()))
                .on_click(move |_| pause(UiAction::SetPaused(!state.paused.get_untracked()))),
            )),
            stack_panel().spacing(12.0).children((
            separator(), controls::label("CREATE A CLASSIC ISLAND"),
            text_box().name("WorldSeed").text(seed_text.two_way()).placeholder_text("World seed"),
            button().content("Generate world").tool_tip_tip("Create a classic island and save your current world")
                .on_click(move |_| {
                    match seed_text.get_untracked().parse::<u64>() {
                        Ok(seed) => regenerate(UiAction::NewWorld(seed)),
                        Err(_) => state.status.set("Enter a whole number for the world seed.".into()),
                    }
                }),
            controls::caption("A seed always creates the same landscape. Use Worlds for open terrain and your saved collection."),
            )),
        ))).into();
    border()
        .is_visible(state.settings_open)
        .child(controls::panel(
            "World settings",
            "MAKE THIS WORLD YOURS",
            content,
            move || state.settings_open.set(false),
            state,
        ))
        .into()
}

pub fn help(state: UiState) -> Element {
    let keys: Vec<Element> = [
        ("DRAG", "Orbit around your construction"),
        ("SHIFT", "Hold while dragging to pan"),
        ("WHEEL", "Zoom toward or away from the world"),
        ("LMB", "Place the selected brick"),
        ("RMB", "Remove the targeted brick"),
        ("R", "Rotate the part by a quarter turn"),
        ("1–8", "Select a hotbar part"),
        ("B", "Open the library; brake while driving"),
        ("TAB", "Switch builder / first-person mode"),
        ("WASD", "Move, drive, or steer"),
        ("SPACE", "Jump or climb when flying"),
        ("Q / E", "Descend / ascend while flying"),
        ("F", "Activate / enter the nearest vehicle"),
        ("HOME", "Return to the overview"),
        ("CTRL Z", "Undo the last construction edit"),
        ("CTRL Y", "Redo an edit"),
        ("CTRL S", "Save the world in your browser"),
        ("ESC", "Close panels, leave vehicle, or pause"),
    ]
    .into_iter()
    .map(|(key, description)| {
        grid()
            .column_definitions("68,*")
            .margin((0.0, 0.0, 0.0, 9.0))
            .children((
                border()
                    .horizontal_alignment(H::Left)
                    .vertical_alignment(V::Top)
                    .child(controls::keycap(key)),
                text_block()
                    .grid_column(1)
                    .text(description)
                    .font_size(11.0)
                    .foreground(theme::MUTED)
                    .text_wrapping(TextWrapping::Wrap),
            ))
            .into_element()
    })
    .collect();
    let content = scroll_viewer().horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(12.0).children((
            controls::caption("Start in builder mode. Click a surface to snap a brick into place. Drag to look around, and use the wheel to get closer."),
            separator(), stack_panel().children(keys), separator(),
            text_block().text("Connected by design").font_size(16.0).font_weight(FontWeight::SemiBold),
            controls::caption("Horizontal positions follow the stud grid. Vertical positions use plate units: three plates make one full brick. Axles, pins, gears and hinges provide mechanical connections."),
            controls::caption("On a touch screen, tap a surface to aim. Drag to orbit; move two fingers together to pan, and pinch to zoom. Place brick and Remove edit the target explicitly. Hold the movement pad with one thumb while looking around with the other. Brake and Park & exit appear while driving."),
        ))).into();
    border()
        .is_visible(state.help_open)
        .child(controls::panel(
            "How to play",
            "SMALL PIECES. BIG IDEAS.",
            content,
            move || state.help_open.set(false),
            state,
        ))
        .into()
}

fn separator() -> Element {
    border()
        .height(1.0)
        .background(theme::EDGE)
        .margin((0.0, 3.0))
        .into()
}
