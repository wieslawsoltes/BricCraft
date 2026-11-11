//! One restrained palette for every native control and floating surface.
use ferroui_base::layout::Layoutable;
use ferroui_base::media::{Color, FontWeight, IEffect, ImmutableDropShadowEffect};
use ferroui_base::Visual;
use ferroui_controls::{primitives::TemplatedControl, TextBlock};
use ferroui_view::{prelude::*, StyleBuilder};
use std::rc::Rc;

pub const SURFACE: &str = "#F0111C23";
pub const EDGE: &str = "#35444D";
pub const TEXT: &str = "#F5F2E9";
pub const MUTED: &str = "#AABBC3";
pub const ACCENT: &str = "#FFAA68";
pub const ACCENT_DARK: &str = "#2C1F17";
pub const INSET: &str = "#1C2A32";

pub fn shadow() -> Option<Rc<dyn IEffect>> {
    Some(Rc::new(ImmutableDropShadowEffect::new(
        0.0,
        8.0,
        28.0,
        Color::from_rgb(0, 0, 0),
        0.24,
    )))
}

pub fn styles() -> Vec<StyleBuilder> {
    vec![
        style("TextBlock")
            .set(TextBlock::foreground_property(), TEXT)
            .set(TextBlock::font_size_property(), 12.0),
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
            .set(TemplatedControl::corner_radius_property(), 8.0)
            .set(TemplatedControl::padding_property(), (12.0, 9.0))
            .set(Layoutable::min_height_property(), 44.0)
            .nested(style("^:pointerover").set(Visual::opacity_property(), 0.86)),
        style("Button.primary")
            .set(TemplatedControl::background_property(), ACCENT)
            .set(TemplatedControl::foreground_property(), ACCENT_DARK)
            .set(TemplatedControl::border_brush_property(), ACCENT),
        style("Button.quiet")
            .set(TemplatedControl::background_property(), "Transparent")
            .set(TemplatedControl::border_brush_property(), "Transparent"),
        style("TextBox")
            .set(TemplatedControl::background_property(), "#14212A")
            .set(TemplatedControl::foreground_property(), TEXT)
            .set(TemplatedControl::border_brush_property(), EDGE)
            .set(TemplatedControl::corner_radius_property(), 8.0)
            .set(TemplatedControl::font_size_property(), 14.0)
            .set(Layoutable::min_height_property(), 44.0),
        style("CheckBox").set(Layoutable::min_height_property(), 44.0),
        style("Slider").set(Layoutable::min_height_property(), 44.0),
    ]
}
