//! Application state, with a fixed-step simulation independent of display refresh.
use crate::vehicle::ActiveVehicle;
use crate::{CameraMode, CameraRig, Input, StreamSession, WorldInfo};
use briccraft_world::{History, Placement, RayHit, World};

pub struct Session {
    pub world: World,
    pub history: History,
    pub streaming: Option<StreamSession>,
    pub world_info: WorldInfo,
    pub gameplay: briccraft_play::Gameplay,
    /// Changes whenever a complete World is replaced, even if its local revision number repeats.
    pub scene_generation: u64,
    pub camera: CameraRig,
    pub input: Input,
    pub selected_part: u16,
    pub color: u8,
    pub rotation: u8,
    pub hit: Option<RayHit>,
    pub preview: Option<Placement>,
    pub active_vehicle: Option<ActiveVehicle>,
    pub last_blueprint: Vec<u32>,
    pub seed: u64,
    pub time: f32,
    pub daylight: f32,
    pub quality: u8,
    pub grid: bool,
    pub sound: bool,
    pub paused: bool,
    pub status: String,
    pub cursor: [f32; 2],
    pub aspect: f32,
    pub(crate) accumulator: f32,
}

impl Session {
    pub fn new(seed: u64) -> Self {
        Self::with_world(World::generate(seed))
    }
    pub(crate) fn with_world(world: World) -> Self {
        let seed = world.seed;
        Self {
            world,
            history: History::default(),
            streaming: None,
            world_info: WorldInfo::classic(seed),
            gameplay: briccraft_play::Gameplay::default(),
            scene_generation: 0,
            camera: CameraRig::default(),
            input: Input::default(),
            selected_part: 3,
            color: 16,
            rotation: 0,
            hit: None,
            preview: None,
            active_vehicle: None,
            last_blueprint: vec![],
            seed,
            time: 0.0,
            daylight: 0.92,
            quality: 1,
            grid: false,
            sound: true,
            paused: false,
            status:
                "Welcome to Sunlit Archipelago. Drag to orbit · scroll to zoom · click to build."
                    .into(),
            cursor: [0.0, 0.0],
            aspect: 1.7,
            accumulator: 0.0,
        }
    }
    pub fn tick(&mut self, elapsed: f32) {
        self.advance_streaming();
        if self.paused {
            self.input.clear();
            return;
        }
        self.accumulator += elapsed.clamp(0.0, 0.1);
        const STEP: f32 = 1.0 / 60.0;
        while self.accumulator >= STEP {
            self.accumulator -= STEP;
            self.time += STEP;
            let mut travel = None;
            if let Some(active) = &mut self.active_vehicle {
                let previous = active.vehicle.position;
                let axes = self.input.axes();
                let controls = briccraft_mechanics::VehicleControl {
                    throttle: axes[2],
                    steering: axes[0],
                    lift: axes[1],
                    roll: 0.0,
                    brake: self.input.brake,
                };
                let world = &self.world;
                let ids = &active.instance_ids;
                active.vehicle.step(STEP, controls, |x, z| {
                    world.surface_height_excluding(x, z, ids)
                });
                // Classic islands retain their finite-world guard. Streamed vehicles cross windows.
                // Flight height remains unrestricted; parking validates the destination's Y cells.
                let (sin, cos) = active.render_yaw().sin_cos();
                let [half_x, half_z] = active.half_extents_xz;
                let half = [
                    cos.abs() * half_x + sin.abs() * half_z,
                    sin.abs() * half_x + cos.abs() * half_z,
                ];
                let dimensions = world.dimensions();
                let mut at_edge = false;
                if self.streaming.is_none() {
                    for (index, axis) in [0, 2].into_iter().enumerate() {
                        let original = active.vehicle.position[axis];
                        active.vehicle.position[axis] =
                            original.clamp(half[index], dimensions[axis] as f32 - half[index]);
                        at_edge |= (original - active.vehicle.position[axis]).abs() > 0.00001;
                    }
                }
                if at_edge {
                    active.vehicle.speed = 0.0;
                    active.vehicle.velocity[0] = 0.0;
                    active.vehicle.velocity[2] = 0.0;
                    self.status = "World edge · turn or reverse to continue. Your construction remains inside the build area.".into();
                }
                let current = active.vehicle.position;
                let horizontal = ((current[0] - previous[0]).powi(2)
                    + (current[2] - previous[2]).powi(2))
                .sqrt();
                let airborne = active.vehicle.kind == briccraft_mechanics::VehicleKind::Plane
                    && current[1]
                        > world.surface_height_excluding(current[0], current[2], ids) + 1.5;
                travel = Some((
                    if airborne {
                        briccraft_play::TravelKind::Flight
                    } else {
                        briccraft_play::TravelKind::Drive
                    },
                    if airborne {
                        (horizontal.powi(2) + (current[1] - previous[1]).powi(2)).sqrt()
                    } else {
                        horizontal
                    },
                ));
                self.camera
                    .follow(active.vehicle.position, active.vehicle.yaw, STEP);
            } else {
                let previous = self.camera.position;
                self.camera.tick_world(STEP, &self.input, &self.world);
                if self.camera.mode == CameraMode::Walk {
                    let delta = self.camera.position - previous;
                    travel = Some((
                        briccraft_play::TravelKind::Walk,
                        (delta.x * delta.x + delta.z * delta.z).sqrt(),
                    ));
                }
            }
            if let Some((kind, distance)) = travel {
                let reward = self.gameplay.record(briccraft_play::Activity::Travel {
                    kind,
                    studs: distance as f64,
                });
                let message = crate::gameplay::reward_text(reward);
                if !message.is_empty() {
                    self.status = message.trim().into();
                }
            }
        }
        self.update_pick();
    }
    pub fn toggle_explore(&mut self) {
        let next = match self.camera.mode {
            CameraMode::Orbit => CameraMode::Fly,
            CameraMode::Fly => CameraMode::Walk,
            CameraMode::Walk => CameraMode::Orbit,
        };
        self.camera.set_mode(next);
        self.status = match next {
            CameraMode::Orbit => "Build view · drag to orbit, shift-drag to pan.",
            CameraMode::Fly => {
                "Free flight · W A S D move, E / Q up and down, Shift boosts. Drag to look."
            }
            CameraMode::Walk => "Walk · W A S D move, Space jumps, Shift runs. Drag to look.",
        }
        .into();
    }
    pub fn mode_name(&self) -> &str {
        if self.active_vehicle.is_some() {
            "DRIVE"
        } else {
            match self.camera.mode {
                CameraMode::Orbit => "BUILD",
                CameraMode::Fly => "FLY",
                CameraMode::Walk => "WALK",
            }
        }
    }
    pub fn select_part(&mut self, id: u16) {
        if briccraft_parts::part(id).is_some() {
            self.selected_part = id;
            self.update_pick();
        }
    }
    pub fn select_color(&mut self, color: u8) {
        if usize::from(color) < briccraft_parts::PALETTE.len() {
            self.color = color;
        }
    }
    pub fn rotate(&mut self) {
        self.rotation = (self.rotation + 1) % 4;
        self.update_pick();
    }
    pub fn undo(&mut self) {
        if self.active_vehicle.is_some() {
            self.status = "Park your vehicle before changing construction history.".into();
            return;
        }
        if self.streaming.is_some() {
            self.change_streamed_history(false);
            return;
        }
        let permit = match self.approve_local_history(false) {
            Ok(permit) => permit,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        self.status = match self.history.undo(&mut self.world) {
            Ok(true) => {
                self.gameplay
                    .commit(permit.expect("history exists"))
                    .expect("approved undo wallet");
                "Construction undone. Resource change reversed; objective rewards are never replayed.".into()
            }
            Ok(false) => "No earlier construction to undo.".into(),
            Err(e) => e.to_string(),
        };
        self.update_pick();
    }
    pub fn redo(&mut self) {
        if self.active_vehicle.is_some() {
            self.status = "Park your vehicle before changing construction history.".into();
            return;
        }
        if self.streaming.is_some() {
            self.change_streamed_history(true);
            return;
        }
        let permit = match self.approve_local_history(true) {
            Ok(permit) => permit,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        self.status = match self.history.redo(&mut self.world) {
            Ok(true) => {
                self.gameplay
                    .commit(permit.expect("history exists"))
                    .expect("approved redo wallet");
                "Construction restored. Resources accounted once; objective rewards are never replayed.".into()
            }
            Ok(false) => "No construction to redo.".into(),
            Err(e) => e.to_string(),
        };
        self.update_pick();
    }
}
