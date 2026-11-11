use crate::atlas::{AtlasError, AtlasStats, PixelAtlas, SceneAtlas};
use crate::camera::normalize;
use crate::{BrickWorldEffect, Camera};
use briccraft_world::World;
use ferroui_base::media::effects::{EffectImage, IEffect, ImmutableCustomEffect};
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::{PixelSize, Rect, Size, Vector};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct PlacementPreview {
    pub position: [i32; 3],
    pub size: [u8; 3],
    pub valid: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VehicleRender {
    pub instance_ids: Vec<u32>,
    /// Pivot in the saved construction's world coordinates (Y in world units).
    pub pivot: [f32; 3],
    /// Current pivot position in world units.
    pub position: [f32; 3],
    /// Euler angles in radians, applied roll, pitch, then yaw. Positive pitch
    /// raises the nose (local positive Z), matching the flight simulation.
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
    pub wheel_angle: f32,
    pub engine_rpm: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RenderFrame {
    pub camera: Camera,
    pub time: f32,
    /// 0 = moonlit, 1 = sunny. Lighting changes without rebuilding geometry.
    pub daylight: f32,
    pub water_level: f32,
    pub selected_id: Option<u32>,
    pub preview: Option<PlacementPreview>,
    /// 0 = fast, 1 = balanced, 2 = full shadows.
    pub quality: u8,
    pub grid: bool,
    pub vehicle: Option<VehicleRender>,
}

impl Default for RenderFrame {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            time: 0.0,
            daylight: 1.0,
            water_level: 2.4,
            selected_id: None,
            preview: None,
            quality: 1,
            grid: false,
            vehicle: None,
        }
    }
}

#[derive(Default)]
pub struct Renderer {
    revision: Option<u64>,
    members: Vec<u32>,
    atlas: Option<SceneAtlas>,
    cells: EffectImage,
    parts: EffectImage,
    uploads: u64,
}

impl Renderer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stats(&self) -> AtlasStats {
        self.atlas
            .as_ref()
            .map(|atlas| AtlasStats {
                uploads: self.uploads,
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
        let members = frame
            .vehicle
            .as_ref()
            .map(|v| v.instance_ids.as_slice())
            .unwrap_or_default();
        if self.revision != Some(world.revision())
            || members != self.members
            || self.atlas.is_none()
        {
            let atlas = SceneAtlas::build(world, members)?;
            self.cells = upload(&atlas.cells);
            self.parts = upload(&atlas.parts);
            self.members = members.to_vec();
            self.revision = Some(world.revision());
            self.atlas = Some(atlas);
            self.uploads += 2;
        }
        let atlas = self.atlas.as_ref().expect("atlas initialized above");
        let dims = world.dimensions();
        let camera = frame.camera;
        let mut effect = BrickWorldEffect {
            world_dimensions: vector4(dims.map(f32::from), atlas.coarse_offset as f32),
            cell_texture_size: Size::new(atlas.cells.width as f64, atlas.cells.height as f64),
            part_texture_size: Size::new(atlas.parts.width as f64, atlas.parts.height as f64),
            camera_position: vector4(camera.position, frame.time),
            camera_forward: vector4(
                camera.forward(),
                (camera.vertical_fov.clamp(0.25, 2.5) * 0.5).tan(),
            ),
            camera_right: vector4(camera.right(), 0.0),
            camera_up: vector4(camera.up(), frame.daylight.clamp(0.0, 1.0)),
            sun_direction: vector4(normalize([-0.45, 0.83, -0.32]), frame.water_level),
            selection: frame
                .selected_id
                .and_then(|id| atlas.packed_ids.get(&id))
                .copied()
                .unwrap_or(0) as i32,
            quality: i32::from(frame.quality.min(2)),
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
        if let Some(vehicle) = &frame.vehicle {
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
                f64::from(
                    (frame.time * vehicle.engine_rpm * std::f32::consts::TAU / 60.0)
                        .rem_euclid(std::f32::consts::TAU),
                ),
            );
        }
        Ok(ImmutableCustomEffect::new(effect).into())
    }

    /// Forces a new upload when switching to another world with the same revision.
    pub fn invalidate(&mut self) {
        self.revision = None;
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

fn rotation_axes(yaw: f32, pitch: f32, roll: f32) -> [[f32; 3]; 3] {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = (-pitch).sin_cos();
    let (sr, cr) = roll.sin_cos();
    [
        [cy * cr + sy * sp * sr, cp * sr, -sy * cr + cy * sp * sr],
        [-cy * sr + sy * sp * cr, cp * cr, sy * sr + cy * sp * cr],
        [sy * cp, -sp, cy * cp],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positive_pitch_raises_the_nose_and_rotation_preserves_lengths() {
        let axes = rotation_axes(0.7, 0.35, -0.2);
        assert!(axes[2][1] > 0.0);
        for axis in axes {
            assert!((axis.iter().map(|v| v * v).sum::<f32>() - 1.0).abs() < 1e-6);
        }
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            assert!(
                axes[a]
                    .iter()
                    .zip(axes[b])
                    .map(|(x, y)| x * y)
                    .sum::<f32>()
                    .abs()
                    < 1e-6
            );
        }
    }
}
