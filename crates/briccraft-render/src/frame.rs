use crate::Camera;
use briccraft_world::Instance;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct PlacementPreview {
    pub position: [i32; 3],
    pub size: [u8; 3],
    pub valid: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VehicleRender {
    /// IDs to exclude from the current static World. Clear or remap these when
    /// replacing its window; detached body IDs may collide with new World IDs.
    pub instance_ids: Vec<u32>,
    /// Independent body geometry retained across streaming window changes.
    /// None resolves `instance_ids` from World as before. Reuse the Rc between
    /// frames; replace it only when the body changes. Its construction coordinate
    /// frame can stay fixed while the active World and vehicle position rebase.
    pub parts: Option<Rc<[Instance]>>,
    /// Pivot in the body's construction coordinates (Y in world units).
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
    /// Decorative infinite ocean height. A negative value disables the plane;
    /// mapped water remains ordinary scene geometry at its measured elevation.
    pub water_level: f32,
    pub selected_id: Option<u32>,
    pub preview: Option<PlacementPreview>,
    /// 0 = fast, 1 = balanced, 2 = full shadows.
    pub quality: u8,
    /// Deterministic spatial samples per physical world pixel: 1, 2 or 4.
    /// Prefer adequate physical resolution before increasing this budget.
    /// Other values round down to the nearest supported count, minimum 1.
    pub samples: u8,
    /// Width in studs of the streamed-window atmospheric boundary. Zero keeps
    /// the classic island unchanged; moving constructions remain visible.
    pub window_fade: f32,
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
            samples: 1,
            window_fade: 0.0,
            grid: false,
            vehicle: None,
        }
    }
}

pub(crate) const fn sample_count(requested: u8) -> u8 {
    match requested {
        0 | 1 => 1,
        2 | 3 => 2,
        _ => 4,
    }
}
