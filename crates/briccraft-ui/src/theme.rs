//! Shared native color, type and interaction treatment for the game interface.
use ferroui_base::layout::Layoutable;
use ferroui_base::media::{Color, FontWeight, IEffect, ImmutableDropShadowEffect};
use ferroui_base::Visual;
use ferroui_controls::{primitives::TemplatedControl, TextBlock};
use ferroui_view::{prelude::*, StyleBuilder};
use std::rc::Rc;

pub const SURFACE: &str = "#F2111B29";
pub const HUD: &str = "#DE101D2B";
pub const EDGE: &str = "#344A5E";
pub const TEXT: &str = "#F4F7FB";
pub const MUTED: &str = "#AEC0D0";
pub const ACCENT: &str = "#FFC36D";
pub const ACCENT_DARK: &str = "#32220F";
pub const INSET: &str = "#1B2B3D";
pub const SELECTED: &str = "#463827";
pub const POSITIVE: &str = "#9CE1C8";

pub fn shadow() -> Option<Rc<dyn IEffect>> {
    Some(Rc::new(ImmutableDropShadowEffect::new(
        0.0,
        5.0,
        18.0,
        Color::from_rgb(0, 0, 0),
        0.22,
    )))
}

pub fn styles() -> Vec<StyleBuilder> {
    vec![
        style("TextBlock")
            .set(TextBlock::foreground_property(), TEXT)
            .set(TextBlock::font_size_property(), 13.0),
        style("Button")
            .set(TemplatedControl::foreground_property(), TEXT)
            .set(TemplatedControl::font_size_property(), 12.0)
            .set(
                TemplatedControl::font_weight_property(),
                FontWeight::SemiBold,
            )
            .set(TemplatedControl::background_property(), INSET)
            .set(TemplatedControl::border_brush_property(), EDGE)
            .set(TemplatedControl::border_thickness_property(), 1.0)
            .set(TemplatedControl::corner_radius_property(), 11.0)
            .set(TemplatedControl::padding_property(), (12.0, 10.0))
            .set(Layoutable::min_height_property(), 44.0)
            .nested(style("^:pointerover").set(Visual::opacity_property(), 0.92))
            .nested(style("^:pressed").set(Visual::opacity_property(), 0.76))
            .nested(style("^:disabled").set(Visual::opacity_property(), 0.45))
            .nested(
                style("^:focus-visible").set(TemplatedControl::border_brush_property(), POSITIVE),
            ),
        style("Button.primary")
            .set(TemplatedControl::background_property(), ACCENT)
            .set(TemplatedControl::foreground_property(), ACCENT_DARK)
            .set(TemplatedControl::border_brush_property(), ACCENT),
        style("Button.quiet")
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::border_brush_property(), "Transparent"),
        style("Button.tool")
            .set(TemplatedControl::background_property(), "#1A2B3C")
            .set(TemplatedControl::border_brush_property(), "#40576A")
            .set(TemplatedControl::padding_property(), 0.0),
        style("TextBox")
            .set(TemplatedControl::background_property(), "#101F30")
            .set(TemplatedControl::foreground_property(), TEXT)
            .set(TemplatedControl::border_brush_property(), EDGE)
            .set(TemplatedControl::corner_radius_property(), 10.0)
            .set(TemplatedControl::font_size_property(), 14.0)
            .set(Layoutable::min_height_property(), 44.0),
        style("CheckBox").set(Layoutable::min_height_property(), 44.0),
        style("Slider").set(Layoutable::min_height_property(), 44.0),
    ]
}
