//! Application state, with a fixed-step simulation independent of display refresh.
use crate::vehicle::ActiveVehicle;
use crate::{CameraMode, CameraRig, Input};
use briccraft_world::{History, Placement, RayHit, World};

pub struct Session {
    pub world: World,
    pub history: History,
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
    accumulator: f32,
}

impl Session {
    pub fn new(seed: u64) -> Self {
        Self {
            world: World::generate(seed),
            history: History::default(),
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
        if self.paused {
            self.input.clear();
            return;
        }
        self.accumulator += elapsed.clamp(0.0, 0.1);
        const STEP: f32 = 1.0 / 60.0;
        while self.accumulator >= STEP {
            self.accumulator -= STEP;
            self.time += STEP;
            if let Some(active) = &mut self.active_vehicle {
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
                self.camera
                    .follow(active.vehicle.position, active.vehicle.yaw, STEP);
            } else {
                let world = &self.world;
                self.camera.tick(STEP, &self.input, |x, z| {
                    world.surface_height(x, z).max(2.4)
                });
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
        self.status = match self.history.undo(&mut self.world) {
            Ok(true) => "Construction undone.".into(),
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
        self.status = match self.history.redo(&mut self.world) {
            Ok(true) => "Construction restored.".into(),
            Ok(false) => "No construction to redo.".into(),
            Err(e) => e.to_string(),
        };
        self.update_pick();
    }
}
