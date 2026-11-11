//! Reusable constructions can be captured, renamed, placed and shared as files.
use crate::{
    controls,
    icons::{self, Icon},
    theme, Dispatch, SavedDesignCard, UiAction, UiState,
};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

fn editor(
    state: UiState,
    name: Signal<String>,
    editing: Signal<Option<u32>>,
    dispatch: Dispatch,
) -> Element {
    let submit = dispatch.clone();
    let import = dispatch;
    border()
        .background(theme::INSET)
        .corner_radius(14.0)
        .padding(14.0)
        .child(
            stack_panel().spacing(10.0).children((
                grid().column_definitions("*,Auto").children((
                    text_block()
                        .text(move || {
                            if editing.get().is_some() {
                                "Rename your design"
                            } else {
                                "Save your construction"
                            }
                        })
                        .font_size(15.0)
                        .font_weight(FontWeight::SemiBold)
                        .text_wrapping(TextWrapping::Wrap)
                        .margin((0.0, 0.0, 8.0, 0.0))
                        .vertical_alignment(V::Center),
                    button()
                        .name("ImportDesign")
                        .grid_column(1)
                        .class("quiet")
                        .padding((8.0, 6.0))
                        .tool_tip_tip("Import a .bricdesign file")
                        .child(icons::label(Icon::Import, "Import", theme::TEXT))
                        .on_click(move |_| import(UiAction::ImportDesign)),
                )),
                text_block()
                    .text(move || {
                        if editing.get().is_some() {
                            "Choose a name you can find easily later.".into()
                        } else {
                            state.designs.get().capture_detail
                        }
                    })
                    .font_size(12.0)
                    .foreground(theme::MUTED)
                    .text_wrapping(TextWrapping::Wrap),
                text_box()
                    .name("DesignName")
                    .text(name.two_way())
                    .placeholder_text("Give it a name, e.g. Rover Mk II"),
                text_block()
                    .text(move || {
                        let count = name.get().trim().chars().count();
                        if count > 64 {
                            "Use a name with at most 64 characters."
                        } else {
                            "Saved with this world. Export a design to use it in another world."
                        }
                    })
                    .font_size(11.0)
                    .foreground(theme::MUTED)
                    .text_wrapping(TextWrapping::Wrap),
                button()
                    .name("SaveDesign")
                    .class("primary")
                    .content(move || {
                        if editing.get().is_some() {
                            "Save name"
                        } else {
                            "Save construction"
                        }
                    })
                    .is_enabled(move || {
                        let value = name.get();
                        let length = value.trim().chars().count();
                        length > 0
                            && length <= 64
                            && state.mode.get() != "DRIVE"
                            && (editing.get().is_some() || state.designs.get().capture_ready)
                    })
                    .on_click(move |_| {
                        let value = name.get_untracked().trim().to_string();
                        if let Some(id) = editing.get_untracked() {
                            submit(UiAction::RenameDesign { id, name: value });
                        } else {
                            submit(UiAction::CaptureDesign(value));
                        }
                    }),
                button()
                    .class("quiet")
                    .content("Finish renaming")
                    .is_visible(move || editing.get().is_some())
                    .on_click(move |_| {
                        editing.set(None);
                        name.set(String::new());
                    }),
            )),
        )
        .into()
}

fn card(
    design: SavedDesignCard,
    state: UiState,
    name: Signal<String>,
    editing: Signal<Option<u32>>,
    deleting: Signal<Option<u32>>,
    dispatch: Dispatch,
) -> Element {
    let id = design.id;
    let rename_name = design.name.clone();
    let place = dispatch.clone();
    let export = dispatch.clone();
    let remove = dispatch;
    let affordable = design.affordable;
    let cost = design.studs;
    border().name(format!("SavedDesign{id}")).background(theme::INSET)
        .border_brush(theme::EDGE).border_thickness(1.0).corner_radius(14.0).padding(14.0)
        .child(stack_panel().spacing(10.0).children((
            grid().column_definitions("Auto,*").children((
                border().width(44.0).height(44.0).corner_radius(11.0).background("#233C50")
                    .vertical_alignment(V::Center).child(icons::glyph(Icon::Plans, theme::POSITIVE)),
                stack_panel().grid_column(1).margin((10.0, 0.0, 0.0, 0.0)).spacing(4.0)
                    .vertical_alignment(V::Center).children((
                        text_block().text(design.name).font_size(14.0).font_weight(FontWeight::SemiBold)
                            .text_wrapping(TextWrapping::Wrap),
                        text_block().text(format!("{} parts · {} × {} studs · {} plates",
                            design.parts, design.size[0], design.size[2], design.size[1]))
                            .font_size(11.0).foreground(theme::MUTED).text_wrapping(TextWrapping::Wrap),
                    )),
            )),
            button().name(format!("PlaceDesign{id}")).class("primary")
                .content(move || if state.gameplay.get().expedition {
                    format!("Place design · {cost} Studs")
                } else {
                    "Place design".into()
                })
                .is_enabled(move || affordable && state.mode.get() != "DRIVE")
                .on_click(move |_| {
                    place(UiAction::PlaceDesign(id));
                }),
            text_block().text("More Studs are needed before placing this design.")
                .font_size(11.0).foreground("#FFBEA3")
                .is_visible(move || state.gameplay.get().expedition && !affordable)
                .text_wrapping(TextWrapping::Wrap),
            grid().column_definitions("*,*,*").children((
                button().name(format!("RenameDesign{id}")).class("quiet").content("Rename")
                    .padding((4.0, 8.0)).font_size(11.0)
                    .on_click(move |_| { editing.set(Some(id)); name.set(rename_name.clone()); deleting.set(None); }),
                button().name(format!("ExportDesign{id}")).grid_column(1).class("quiet").content("Export")
                    .padding((4.0, 8.0)).font_size(11.0)
                    .tool_tip_tip("Download this construction as a .bricdesign file")
                    .on_click(move |_| export(UiAction::ExportDesign(id))),
                button().name(format!("DeleteDesign{id}")).grid_column(2).class("quiet").content("Delete")
                    .padding((4.0, 8.0)).font_size(11.0)
                    .on_click(move |_| deleting.set(Some(id))),
            )),
            border().background("#3E2930").corner_radius(10.0).padding(11.0)
                .is_visible(move || deleting.get() == Some(id))
                .child(stack_panel().spacing(10.0).children((
                    controls::caption("Delete this saved design? Constructions already placed in the world stay."),
                    grid().column_definitions("*,*").children((
                        button().content("Keep design").font_size(11.0).padding((5.0, 8.0)).margin((0.0, 0.0, 4.0, 0.0))
                            .on_click(move |_| deleting.set(None)),
                        button().name(format!("ConfirmDeleteDesign{id}")).grid_column(1)
                            .content("Delete design").font_size(11.0).padding((5.0, 8.0)).margin((4.0, 0.0, 0.0, 0.0))
                            .background("#6C3842").border_brush("#A86772")
                            .on_click(move |_| {
                                remove(UiAction::DeleteDesign(id));
                                deleting.set(None);
                                if editing.get_untracked() == Some(id) { editing.set(None); }
                            }),
                    )),
                ))),
        ))).into()
}

fn world_view(state: UiState, dispatch: Dispatch) -> Element {
    let name = signal(String::new());
    let editing = signal(None::<u32>);
    let deleting = signal(None::<u32>);
    let cards_dispatch = dispatch.clone();
    scroll_viewer()
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(14.0).children((
            controls::caption("Capture your own construction once, then place copies with the same parts, colors and rotations. Imported designs join this world's library."),
            editor(state, name, editing, dispatch),
            rerender(move || {
                let library = state.designs.get();
                if library.designs.is_empty() {
                    border().background("#152434").corner_radius(14.0).padding(18.0)
                        .child(stack_panel().spacing(9.0).children((
                            border().horizontal_alignment(H::Left)
                                .child(icons::glyph(Icon::Plans, theme::POSITIVE)),
                            text_block().text("Your next great idea starts here").font_size(15.0)
                                .font_weight(FontWeight::SemiBold).text_wrapping(TextWrapping::Wrap),
                            controls::caption("Point at a separate construction, give it a name above, and save it. Or import a design shared from another world."),
                        ))).into_element()
                } else {
                    let cards: Vec<Element> = library.designs.into_iter().map(|design| {
                        card(design, state, name, editing, deleting, cards_dispatch.clone())
                    }).collect();
                    stack_panel().spacing(10.0).children(cards).into_element()
                }
            }),
            border().background("#142D32").corner_radius(10.0).padding(12.0)
                .child(stack_panel().spacing(6.0).children((
                    controls::label("LATEST ACTION"),
                    text_block().text(state.status).font_size(11.0).foreground(theme::POSITIVE)
                        .text_wrapping(TextWrapping::Wrap),
                ))),
        ))).into()
}

/// Names and pending confirmations belong to one world's design identifiers.
/// Replacing that world also replaces this view's local reactive owner.
pub fn view(state: UiState, dispatch: Dispatch) -> Element {
    rerender(move || {
        let _world_id = state.world_id.get();
        world_view(state, dispatch.clone())
    })
    .into()
}
