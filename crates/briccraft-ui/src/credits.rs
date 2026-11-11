//! Persistent source credit expands to complete, scrollable native attribution.
use crate::{controls, theme, Dispatch, UiAction, UiState};
use ferroui_base::layout::{HorizontalAlignment as H, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, TextTrimming, TextWrapping};
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_view::prelude::*;

pub fn badge(state: UiState) -> Element {
    button()
        .name("WorldAttribution")
        .class("quiet")
        .horizontal_alignment(H::Right)
        .vertical_alignment(move || {
            if state.compact.get() {
                V::Top
            } else {
                V::Bottom
            }
        })
        .margin(move || {
            if state.compact.get() {
                (12.0, state.layout.get().header_bottom() + 54.0, 12.0, 0.0)
            } else {
                (16.0, 0.0, 16.0, 56.0)
            }
        })
        .max_width(move || (state.layout.get().width - 24.0).clamp(1.0, 420.0))
        .padding((10.0, 6.0))
        .background(theme::HUD)
        .is_visible(move || !state.attribution.get().is_empty() && !state.layout.get().keyboard)
        .tool_tip_tip("View all data sources, credits and licenses")
        .child(
            text_block()
                .text(state.attribution)
                .font_size(10.0)
                .foreground("#D7E3E7")
                .text_wrapping(TextWrapping::Wrap)
                .max_lines(2)
                .text_trimming(<dyn TextTrimming>::character_ellipsis()),
        )
        .on_click(move |_| {
            state.close_panels();
            state.credits_open.set(true);
        })
        .into()
}

pub fn panel(state: UiState, dispatch: Dispatch) -> Element {
    let sources = dispatch.clone();
    let content = scroll_viewer()
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .child(stack_panel().spacing(14.0).children((
            controls::caption("These credits travel with your saved world. The source report also records geometry conversion, inferred heights and terrain scaling."),
            rerender(move || {
                let credits: Vec<Element> = state.credits.get().into_iter().map(|source| {
                    let open = sources.clone();
                    let url = source.url.clone();
                    border().padding(14.0).corner_radius(14.0).background(theme::INSET)
                        .child(stack_panel().spacing(10.0).children((
                            text_block().text(source.label).font_size(13.0).font_weight(FontWeight::SemiBold)
                                .text_wrapping(TextWrapping::Wrap),
                            text_block().text(format!("License: {}", source.license)).font_size(12.0)
                                .foreground(theme::MUTED).text_wrapping(TextWrapping::Wrap),
                            button().content("Open source details").is_visible(!url.is_empty())
                                .on_click(move |_| open(UiAction::OpenSource(url.clone()))),
                        ))).into_element()
                }).collect();
                stack_panel().spacing(10.0).children(credits).into_element()
            }),
            button().content("Export source report").on_click(move |_| dispatch(UiAction::ExportReport)),
        ))).into();
    border()
        .is_visible(state.credits_open)
        .child(controls::panel(
            "Source credits",
            "REAL DATA. DOCUMENTED ORIGINS.",
            content,
            move || state.credits_open.set(false),
            state,
        ))
        .into()
}
