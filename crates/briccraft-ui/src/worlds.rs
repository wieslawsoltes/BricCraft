//! A native world library: creation, snapshots, and reversible archiving.
use crate::{controls, theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, Orientation};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

pub fn panel(state: UiState, dispatch: Dispatch) -> Element {
    let name = signal(String::from("My brick world"));
    let seed = signal(String::from("42"));
    let preset = signal(String::from("meadow"));
    let show_archive = signal(false);
    let create = dispatch.clone();
    let duplicate = dispatch.clone();
    let saved = dispatch.clone();
    let map = dispatch.clone();
    let choices: Vec<Element> = [
        ("meadow", "Meadow", "Rolling hills & woodland", "#79AF75"),
        ("alpine", "Alpine", "High ridges & snow", "#BACDDA"),
        ("desert", "Desert", "Dunes & warm stone", "#DDB872"),
        ("coast", "Coast", "Waterways & shorelines", "#61B6BF"),
        ("flat", "Studio", "An open construction plain", "#B2BC9A"),
        (
            "island",
            "Classic island",
            "Village, rail loop & airstrip",
            "#EDAA75",
        ),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (id, title, hint, color))| {
        border()
            .grid_row((index / 2) as i32)
            .grid_column((index % 2) as i32)
            .margin((3.0, 3.0, 3.0, 3.0))
            .child(
                button()
                    .name(format!("Preset{id}"))
                    .horizontal_content_alignment(H::Stretch)
                    .padding(10.0)
                    .background(move || {
                        if preset.get() == id {
                            "#32454A"
                        } else {
                            theme::INSET
                        }
                    })
                    .border_brush(move || {
                        if preset.get() == id {
                            theme::ACCENT
                        } else {
                            theme::EDGE
                        }
                    })
                    .on_click(move |_| preset.set(id.into()))
                    .child(
                        stack_panel().spacing(6.0).children((
                            border().height(5.0).corner_radius(2.0).background(color),
                            text_block()
                                .text(title)
                                .font_size(13.0)
                                .font_weight(FontWeight::SemiBold),
                            text_block()
                                .text(hint)
                                .font_size(10.0)
                                .foreground(theme::MUTED)
                                .text_wrapping(TextWrapping::Wrap),
                        )),
                    ),
            )
            .into_element()
    })
    .collect();
    let content = scroll_viewer()
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(12.0).children((
            stack_panel().spacing(12.0).children((
            controls::caption("Keep a collection of worlds. Open terrain streams around you as you travel; your constructions stay where you built them."),
            controls::label("CREATE A WORLD"),
            text_box().name("NewWorldName").placeholder_text("World name").text(name.two_way()),
            grid().column_definitions("*,*").row_definitions("Auto,Auto,Auto").children(choices),
            text_box().name("NewWorldSeed").placeholder_text("Seed · a whole number").text(seed.two_way()),
            button().class("primary").content("Create world").on_click(move |_| {
                match seed.get_untracked().trim().parse::<u64>() {
                    Ok(seed) => create(UiAction::CreateWorld {
                        preset:preset.get_untracked(), name:name.get_untracked(), seed,
                    }),
                    Err(_) => state.status.set("Enter a whole number for the seed.".into()),
                }
            }),
            button().content("Build a real place or 3D model").on_click(move |_| {
                state.close_panels(); state.imports_open.set(true);
            }),
            button().class("quiet").content("Visit Brandenburg Gate").on_click(move |_| map(UiAction::ImportLandmark)),
            )),
            border().height(1.0).background(theme::EDGE).margin((0.0,5.0)),
            stack_panel().spacing(12.0).children((
            controls::label("YOUR SAVED WORLDS"),
            controls::caption("The current world is saved before opening or creating another one. Archiving is reversible."),
            button().content("Save a copy with the name above").on_click(move |_| duplicate(UiAction::SaveWorldAs(name.get_untracked()))),
            check_box().content("Show archived worlds").is_checked(move || Some(show_archive.get()))
                .on_click(move |_| show_archive.set(!show_archive.get_untracked())),
            rerender(move || {
                let worlds = state.worlds.get();
                let archived = show_archive.get();
                let current = state.world_id.get();
                let cards: Vec<Element> = worlds.iter().filter(|w| w.archived == archived).map(|world| {
                    let open = saved.clone();
                    let archive = saved.clone();
                    let id = world.id.clone();
                    let archive_id = world.id.clone();
                    let active = id == current;
                    border().padding(12.0).corner_radius(10.0).background(theme::INSET)
                        .border_brush(if active { theme::ACCENT } else { theme::EDGE }).border_thickness(1.0)
                        .child(stack_panel().spacing(7.0).children((
                            text_block().text(format!("{}{}",world.name,if active { " · current" } else { "" }))
                                .font_size(14.0).font_weight(FontWeight::SemiBold).text_wrapping(TextWrapping::Wrap),
                            controls::caption(format!("{}\n{}",world.detail,world.updated)),
                            stack_panel().orientation(Orientation::Horizontal).spacing(5.0).children((
                                button().content("Open").is_enabled(!active).on_click(move |_| open(UiAction::OpenWorld(id.clone()))),
                                button().class("quiet").content(if archived { "Restore" } else { "Archive" })
                                    .is_enabled(!active || archived)
                                    .on_click(move |_| archive(UiAction::ArchiveWorld { id:archive_id.clone(),archived:!archived })),
                            )),
                        ))).into_element()
                }).collect();
                if cards.is_empty() {
                    controls::caption(if archived { "No archived worlds." } else { "Your saved worlds will appear here." })
                } else { stack_panel().spacing(9.0).children(cards).into_element() }
            }),
            )),
        ))).into();
    border()
        .is_visible(state.worlds_open)
        .child(controls::panel(
            "Worlds",
            "AN ENDLESS PLACE TO BUILD",
            content,
            move || state.worlds_open.set(false),
            state,
        ))
        .into()
}
