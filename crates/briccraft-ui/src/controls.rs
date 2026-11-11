use crate::{icons, theme, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextWrapping};
use ferroui_controls::Button;
use ferroui_view::prelude::*;

pub fn label(value: impl Into<String>) -> Element {
    text_block()
        .text(value.into())
        .foreground(theme::MUTED)
        .font_size(10.0)
        .font_weight(FontWeight::SemiBold)
        .into()
}

pub fn caption(value: impl Into<String>) -> Element {
    text_block()
        .text(value.into())
        .foreground(theme::MUTED)
        .text_wrapping(TextWrapping::Wrap)
        .font_size(12.0)
        .into()
}

pub fn tool_button(
    name: &str,
    hint: &str,
    icon: icons::Icon,
    activate: impl Fn() + 'static,
) -> El<Button> {
    button()
        .name(name)
        .class("quiet")
        .width(44.0)
        .height(44.0)
        .padding(0.0)
        .automation_properties_name(Some(hint.to_string()))
        .tool_tip_tip(hint)
        .child(icons::glyph(icon, theme::TEXT))
        .on_click(move |_| activate())
}

pub fn keycap(text: &str) -> Element {
    border()
        .background("#263A4E")
        .border_brush("#536C81")
        .border_thickness((1.0, 1.0, 1.0, 2.0))
        .corner_radius(5.0)
        .padding((6.0, 2.0))
        .child(
            text_block()
                .text(text)
                .font_size(10.0)
                .foreground("#D9E8F3"),
        )
        .into()
}

pub fn panel(
    title: &str,
    eyebrow: &str,
    content: Element,
    close: impl Fn() + 'static,
    state: UiState,
) -> Element {
    border()
        .name(format!("{}Panel", title.replace(' ', "")))
        .width(move || state.layout.get().panel_width())
        .horizontal_alignment(H::Right)
        .vertical_alignment(V::Stretch)
        .margin(move || state.layout.get().panel_margin())
        .background(theme::SURFACE)
        .border_brush("#53697F")
        .border_thickness(1.0)
        .corner_radius(20.0)
        .padding(move || {
            if state.layout.get().landscape {
                14.0
            } else if state.compact.get() {
                16.0
            } else {
                24.0
            }
        })
        .effect(theme::shadow())
        .child(
            grid().row_definitions("Auto,*").children((
                grid()
                    .column_definitions("*,Auto")
                    .margin(move || {
                        (
                            0.0,
                            0.0,
                            0.0,
                            if state.layout.get().landscape {
                                10.0
                            } else {
                                18.0
                            },
                        )
                    })
                    .children((
                        stack_panel()
                            .spacing(6.0)
                            .margin((0.0, 0.0, 8.0, 0.0))
                            .children((
                                border()
                                    .height(3.0)
                                    .width(30.0)
                                    .corner_radius(2.0)
                                    .background(theme::ACCENT)
                                    .horizontal_alignment(H::Left)
                                    .is_visible(move || !state.layout.get().landscape),
                                border()
                                    .is_visible(move || !state.layout.get().landscape)
                                    .child(label(eyebrow)),
                                text_block()
                                    .text(title)
                                    .font_size(
                                        move || if state.compact.get() { 22.0 } else { 25.0 },
                                    )
                                    .font_weight(FontWeight::SemiBold)
                                    .text_wrapping(TextWrapping::Wrap),
                            )),
                        tool_button("ClosePanel", "Close panel · Esc", icons::Icon::Close, close)
                            .grid_column(1)
                            .vertical_alignment(V::Top)
                            .background("#23364A")
                            .border_brush("#40586E"),
                    )),
                border().grid_row(1).child(content),
            )),
        )
        .into()
}

/// A native backdrop owns outside taps before the world can interpret them as edits.
pub fn modal_backdrop(state: UiState) -> Element {
    border()
        .name("PanelBackdrop")
        .background("#8006101C")
        .is_visible(move || state.modal())
        .on_pointer_pressed(move |event| {
            if !state.import_busy.get_untracked() && !state.world_busy.get_untracked() {
                state.close_panels();
            }
            event.set_handled(true);
        })
        .into()
}
