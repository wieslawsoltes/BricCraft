use ferroui_base::media::effects::{
    CustomEffect, EffectImage, EffectParameterValue, ShaderEffectDescription,
};
use ferroui_base::media::imaging::BitmapBlendingMode;
use ferroui_base::{Rect, Size};

/// The SkSL is compiled once by FerroUI and runs inside the Skia backend.
pub const WORLD_SKSL: &str = concat!(
    include_str!("shaders/uniforms.sksl"),
    include_str!("shaders/geometry.sksl"),
    include_str!("shaders/trace.sksl"),
    include_str!("shaders/lighting.sksl"),
    include_str!("shaders/main.sksl"),
);

/// Fast and Balanced are distinct effect types, so FerroUI/Skia compile only
/// their actual call graph. A uniform branch inside the Full shader would
/// still require its nested marcher and normal evaluation to be compiled.
pub const FAST_WORLD_SKSL: &str = concat!(
    include_str!("shaders/uniforms.sksl"),
    include_str!("shaders/geometry_analytic.sksl"),
    include_str!("shaders/trace_primary.sksl"),
    include_str!("shaders/shadows_fast.sksl"),
    include_str!("shaders/lighting_efficient.sksl"),
    include_str!("shaders/main.sksl"),
);

pub const BALANCED_WORLD_SKSL: &str = concat!(
    include_str!("shaders/uniforms.sksl"),
    include_str!("shaders/geometry_analytic.sksl"),
    include_str!("shaders/trace_primary.sksl"),
    include_str!("shaders/shadows_balanced.sksl"),
    include_str!("shaders/lighting_efficient.sksl"),
    include_str!("shaders/main.sksl"),
);

pub const fn shader_variant_name(quality: u8) -> &'static str {
    match quality {
        0 => "analytic-fast",
        1 => "analytic-balanced",
        _ => "full-geometry",
    }
}

/// An immutable snapshot shared with the render thread. `Rect` is the framework's
/// four-float uniform carrier; the members here are vectors, not UI rectangles.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BrickWorldEffect {
    pub world_dimensions: Rect,
    pub scene_bounds_min: Rect,
    pub scene_bounds_max: Rect,
    pub cell_texture_size: Size,
    pub part_texture_size: Size,
    pub camera_position: Rect,
    pub camera_forward: Rect,
    pub camera_right: Rect,
    pub camera_up: Rect,
    pub sun_direction: Rect,
    pub preview_origin: Rect,
    pub preview_size: Rect,
    pub selection: i32,
    pub quality: i32,
    pub construction_grid: i32,
    pub vehicle_dimensions: Rect,
    pub vehicle_position: Rect,
    pub vehicle_axis_x: Rect,
    pub vehicle_axis_y: Rect,
    pub vehicle_axis_z: Rect,
    pub vehicle_motion: Size,
    pub cells: EffectImage,
    pub parts: EffectImage,
}

macro_rules! parameters {
    ($self:ident, $f:ident; $($field:ident),* $(,)?) => { $( $f(stringify!($field), $self.$field.clone().into()); )* };
}

impl CustomEffect for BrickWorldEffect {
    const NAME: &'static str = "briccraft-world";

    fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
        parameters!(self, f;
            world_dimensions, scene_bounds_min, scene_bounds_max, cell_texture_size, part_texture_size,
            camera_position, camera_forward, camera_right, camera_up, sun_direction,
            preview_origin, preview_size, selection, quality, construction_grid, vehicle_dimensions,
            vehicle_position, vehicle_axis_x, vehicle_axis_y, vehicle_axis_z, vehicle_motion, cells, parts
        );
    }

    fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
        macro_rules! rect {
            ($field:ident) => {
                value.as_rect().map(|v| self.$field = v).is_some()
            };
        }
        match name {
            "world_dimensions" => rect!(world_dimensions),
            "scene_bounds_min" => rect!(scene_bounds_min),
            "scene_bounds_max" => rect!(scene_bounds_max),
            "cell_texture_size" => value
                .as_size()
                .map(|v| self.cell_texture_size = v)
                .is_some(),
            "part_texture_size" => value
                .as_size()
                .map(|v| self.part_texture_size = v)
                .is_some(),
            "camera_position" => rect!(camera_position),
            "camera_forward" => rect!(camera_forward),
            "camera_right" => rect!(camera_right),
            "camera_up" => rect!(camera_up),
            "sun_direction" => rect!(sun_direction),
            "preview_origin" => rect!(preview_origin),
            "preview_size" => rect!(preview_size),
            "selection" => value.as_int().map(|v| self.selection = v).is_some(),
            "quality" => value.as_int().map(|v| self.quality = v).is_some(),
            "construction_grid" => value.as_int().map(|v| self.construction_grid = v).is_some(),
            "vehicle_dimensions" => rect!(vehicle_dimensions),
            "vehicle_position" => rect!(vehicle_position),
            "vehicle_axis_x" => rect!(vehicle_axis_x),
            "vehicle_axis_y" => rect!(vehicle_axis_y),
            "vehicle_axis_z" => rect!(vehicle_axis_z),
            "vehicle_motion" => value.as_size().map(|v| self.vehicle_motion = v).is_some(),
            "cells" => value.as_image().map(|v| self.cells = v.clone()).is_some(),
            "parts" => value.as_image().map(|v| self.parts = v.clone()).is_some(),
            _ => false,
        }
    }

    fn shader() -> Option<ShaderEffectDescription> {
        // BricCraft explicitly selects Skia. There is no hidden JavaScript renderer
        // or silent change to a different graphics backend.
        Some(
            ShaderEffectDescription::overlay(WORLD_SKSL, "", BitmapBlendingMode::Source)
                .with_children(["cells", "parts"]),
        )
    }
}

macro_rules! analytic_effect {
    ($name:ident, $source:ident, $label:literal) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub(crate) struct $name(pub BrickWorldEffect);

        impl CustomEffect for $name {
            const NAME: &'static str = $label;

            fn for_each_parameter(&self, f: &mut dyn FnMut(&'static str, EffectParameterValue)) {
                self.0.for_each_parameter(f);
            }

            fn set_parameter(&mut self, name: &str, value: EffectParameterValue) -> bool {
                self.0.set_parameter(name, value)
            }

            fn shader() -> Option<ShaderEffectDescription> {
                Some(
                    ShaderEffectDescription::overlay($source, "", BitmapBlendingMode::Source)
                        .with_children(["cells", "parts"]),
                )
            }
        }
    };
}

analytic_effect!(FastBrickWorldEffect, FAST_WORLD_SKSL, "briccraft-world-fast");
analytic_effect!(BalancedBrickWorldEffect, BALANCED_WORLD_SKSL, "briccraft-world-balanced");
