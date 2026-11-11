use crate::camera::{normalize, rotation_axes};
use crate::effect::shader_variant_name;
use crate::effect_cache::EffectCache;
use crate::frame::sample_count;
use crate::{AtlasStats, BrickWorldEffect, RenderFrame};
use briccraft_render_data::{AtlasError, PixelAtlas, SceneAtlas};
use briccraft_world::{Instance, World};
use ferroui_base::media::effects::{EffectImage, IEffect};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::{PixelSize, Rect, Size, Vector};
use std::rc::Rc;

#[derive(Default)]
pub struct Renderer {
    revision: Option<u64>,
    members: Vec<u32>,
    vehicle_parts: Option<Rc<[Instance]>>,
    atlas: Option<SceneAtlas>,
    cells: EffectImage,
    parts: EffectImage,
    uploads: u64,
    cell_uploads: u64,
    part_uploads: u64,
    uploaded_bytes: u64,
    last_uploaded_bytes: usize,
    rebuilds: u64,
    effect_frames: u64,
    uniform_only_frames: u64,
    last_time: Option<f32>,
    rotor_angle: f32,
    effective_quality: u8,
    effective_samples: u8,
    effects: EffectCache,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Physical-pixel sample count in the most recent effect snapshot.
    pub fn samples(&self) -> u8 {
        self.effective_samples.max(1)
    }

    /// Calls that reused the exact immutable effect and did not publish a frame.
    pub fn reused_effects(&self) -> u64 {
        self.effects.reused()
    }

    pub fn stats(&self) -> AtlasStats {
        self.atlas
            .as_ref()
            .map(|atlas| AtlasStats {
                uploads: self.uploads,
                cell_uploads: self.cell_uploads,
                part_uploads: self.part_uploads,
                uploaded_bytes: self.uploaded_bytes,
                last_uploaded_bytes: self.last_uploaded_bytes,
                rebuilds: self.rebuilds,
                effect_frames: self.effect_frames,
                uniform_only_frames: self.uniform_only_frames,
                shader_quality: self.effective_quality,
                shader_variant: shader_variant_name(self.effective_quality),
                ..atlas.stats
            })
            .unwrap_or_default()
    }

    /// Must be called after FerroUI initializes the Skia render interface.
    pub fn effect(
        &mut self,
        world: &World,
        frame: &RenderFrame,
    ) -> Result<Rc<dyn IEffect>, AtlasError> {
        self.last_uploaded_bytes = 0;
        let dt = self
            .last_time
            .map(|time| (frame.time - time).clamp(0.0, 0.1))
            .unwrap_or(0.0);
        self.last_time = Some(frame.time);
        if let Some(vehicle) = &frame.vehicle {
            self.rotor_angle = (self.rotor_angle
                + dt * vehicle.engine_rpm * std::f32::consts::TAU / 60.0)
                .rem_euclid(std::f32::consts::TAU);
        }
        let members = frame
            .vehicle
            .as_ref()
            .map(|v| v.instance_ids.as_slice())
            .unwrap_or_default();
        let detached = frame.vehicle.as_ref().and_then(|v| v.parts.as_ref());
        if self.revision != Some(world.revision())
            || members != self.members
            || !same_parts(self.vehicle_parts.as_ref(), detached)
            || self.atlas.is_none()
        {
            let atlas = SceneAtlas::build(world, members, detached.map(|p| p.as_ref()))?;
            // Preserve each immutable image independently. Recoloring uploads
            // the small descriptor texture only; an identical restored window
            // uploads neither. Skia's EffectImage API exposes whole images, not
            // partial texture writes, so these counts describe actual new images.
            if self
                .atlas
                .as_ref()
                .is_none_or(|old| old.cells != atlas.cells)
            {
                self.cells = upload(&atlas.cells);
                self.cell_uploads += 1;
                self.uploads += 1;
                self.last_uploaded_bytes += atlas.cells.pixels.len();
            }
            if self
                .atlas
                .as_ref()
                .is_none_or(|old| old.parts != atlas.parts)
            {
                self.parts = upload(&atlas.parts);
                self.part_uploads += 1;
                self.uploads += 1;
                self.last_uploaded_bytes += atlas.parts.pixels.len();
            }
            self.uploaded_bytes += self.last_uploaded_bytes as u64;
            self.rebuilds += 1;
            self.members = members.to_vec();
            self.vehicle_parts = detached.cloned();
            self.revision = Some(world.revision());
            self.atlas = Some(atlas);
        }
        let atlas = self.atlas.as_ref().expect("atlas initialized above");
        let dims = world.dimensions();
        let camera = frame.camera;
        let mut effect = BrickWorldEffect {
            world_dimensions: vector4(dims.map(f32::from), atlas.coarse_offset as f32),
            scene_bounds_min: vector4(
                [
                    f32::from(atlas.static_min[0]),
                    f32::from(atlas.static_min[1]) * 0.4,
                    f32::from(atlas.static_min[2]),
                ],
                atlas.height_offset as f32,
            ),
            scene_bounds_max: vector4(
                [
                    f32::from(atlas.static_max[0]),
                    f32::from(atlas.static_max[1]) * 0.4 + 0.18,
                    f32::from(atlas.static_max[2]),
                ],
                if atlas.stats.static_instances > 0 {
                    1.0
                } else {
                    0.0
                },
            ),
            cell_texture_size: Size::new(atlas.cells.width as f64, atlas.cells.height as f64),
            part_texture_size: Size::new(atlas.parts.width as f64, atlas.parts.height as f64),
            camera_position: vector4(camera.position, frame.time),
            camera_forward: vector4(
                camera.forward(),
                (camera.vertical_fov.clamp(0.25, 2.5) * 0.5).tan(),
            ),
            camera_right: vector4(camera.right(), frame.window_fade.clamp(0.0, 32.0)),
            camera_up: vector4(camera.up(), frame.daylight.clamp(0.0, 1.0)),
            sun_direction: vector4(normalize([-0.45, 0.83, -0.32]), frame.water_level),
            selection: frame
                .selected_id
                .and_then(|id| atlas.packed_ids.get(&id))
                .copied()
                .unwrap_or(0) as i32,
            quality: i32::from(frame.quality.min(2)),
            sample_count: i32::from(sample_count(frame.samples)),
            construction_grid: i32::from(frame.grid),
            cells: self.cells.clone(),
            parts: self.parts.clone(),
            ..BrickWorldEffect::default()
        };
        if let Some(preview) = &frame.preview {
            effect.preview_origin = vector4(
                [
                    preview.position[0] as f32,
                    preview.position[1] as f32 * 0.4,
                    preview.position[2] as f32,
                ],
                1.0,
            );
            effect.preview_size = vector4(
                [
                    f32::from(preview.size[0]),
                    f32::from(preview.size[1]) * 0.4,
                    f32::from(preview.size[2]),
                ],
                if preview.valid { 1.0 } else { 0.0 },
            );
        }
        if let Some(vehicle) = frame
            .vehicle
            .as_ref()
            .filter(|_| atlas.stats.vehicle_instances > 0)
        {
            let pivot = [
                vehicle.pivot[0] - atlas.vehicle_min[0] as f32,
                vehicle.pivot[1] - atlas.vehicle_min[1] as f32 * 0.4,
                vehicle.pivot[2] - atlas.vehicle_min[2] as f32,
            ];
            let axes = rotation_axes(vehicle.yaw, vehicle.pitch, vehicle.roll);
            effect.vehicle_dimensions = vector4(
                atlas.vehicle_size.map(f32::from),
                atlas.vehicle_offset as f32,
            );
            effect.vehicle_position = vector4(vehicle.position, 1.0);
            effect.vehicle_axis_x = vector4(axes[0], pivot[0]);
            effect.vehicle_axis_y = vector4(axes[1], pivot[1]);
            effect.vehicle_axis_z = vector4(axes[2], pivot[2]);
            effect.vehicle_motion = Size::new(
                f64::from(vehicle.wheel_angle.rem_euclid(std::f32::consts::TAU)),
                f64::from(self.rotor_angle),
            );
        }
        self.effective_quality = frame.quality.min(2);
        self.effective_samples = sample_count(frame.samples);
        let (effect, published) = self.effects.resolve(effect);
        if published {
            self.effect_frames += 1;
            self.uniform_only_frames += u64::from(self.last_uploaded_bytes == 0);
        }
        Ok(effect)
    }

    /// Drops only the retained effect snapshot, preserving atlas images and
    /// upload counters. Use when the world layer's presentation target changes.
    pub fn invalidate_effect(&mut self) {
        self.effects.invalidate();
    }

    /// Rebuilds when replacing a World with the same revision. Identical images
    /// remain cached; a camera change alone never requires this call.
    pub fn invalidate(&mut self) {
        self.revision = None;
        self.last_time = None;
        self.invalidate_effect();
    }
}

fn same_parts(a: Option<&Rc<[Instance]>>, b: Option<&Rc<[Instance]>>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => Rc::ptr_eq(a, b) || a.as_ref() == b.as_ref(),
        (None, None) => true,
        _ => false,
    }
}

fn upload(atlas: &PixelAtlas) -> EffectImage {
    let bitmap = Bitmap::from_pixels(
        PixelFormat::RGBA8888,
        AlphaFormat::Opaque,
        &atlas.pixels,
        PixelSize::new(atlas.width as i32, atlas.height as i32),
        Vector::new(96.0, 96.0),
        atlas.width as i32 * 4,
    );
    EffectImage::new(&bitmap)
}

fn vector4(v: [f32; 3], w: f32) -> Rect {
    Rect::new(
        f64::from(v[0]),
        f64::from(v[1]),
        f64::from(v[2]),
        f64::from(w),
    )
}
