//! Continuous, network-fed geographic worlds. The host owns I/O and cancellation;
//! this module owns atomic world publication and safe movement while data is pending.
use crate::{CameraMode, GameAttribution, MapAnchor, MapRegion, Session, StreamSession, WorldInfo};
use briccraft_stream::{BiomePreset, GlobalPos, StreamError, StreamWorld, WindowOrigin};
use glam::Vec3;

impl Session {
    /// Build a complete candidate before a browser replaces its current world.
    /// Source generation is independent of player construction history/resources.
    pub fn new_mapped(
        seed: u64,
        anchor: MapAnchor,
        initial: MapRegion,
        name: String,
        attributions: Vec<GameAttribution>,
        walk: bool,
    ) -> Result<Self, StreamError> {
        anchor.validate()?;
        let focus = GlobalPos::new(i64::from(anchor.origin[0]), 0, i64::from(anchor.origin[2]));
        let origin = WindowOrigin::around(focus)?;
        let mut stream =
            StreamWorld::new_mapped(seed, anchor, briccraft_stream::DEFAULT_CACHE_CAPACITY)?;
        stream.publish_map_region(initial)?;
        let snapshot = stream.prepare_window(origin)?;
        let mut session = Self::with_world(snapshot.world.clone());
        session.world_info = WorldInfo::streamed(seed, BiomePreset::Flat);
        session.world_info.id = format!("mapped-{seed}");
        session.world_info.name = name;
        session.world_info.attributions = attributions;
        crate::persistence::validate_info(&session.world_info)?;
        let center = snapshot.local_position(focus)?;
        let spawn = walking_spawn(&session.world, center[0], center[2]);
        session.camera.focus([spawn[0], spawn[1], spawn[2]], 52.0);
        if walk {
            session.camera.set_mode(CameraMode::Walk);
            session.camera.position = Vec3::from_array(spawn);
            session.camera.pitch = -0.05;
            session.camera.target = session.camera.position + session.camera.forward() * 12.0;
        }
        session.streaming = Some(StreamSession::new(stream, snapshot));
        session.status = if walk {
            "Free roam · walk through real map terrain. Nearby areas load as you move; your constructions stay with this world."
        } else {
            "Mapped building world · build here, or switch to Walk to explore surrounding real-world areas."
        }.into();
        session.update_pick();
        Ok(session)
    }

    pub fn map_anchor(&self) -> Option<&MapAnchor> {
        self.streaming.as_ref()?.stream.map_anchor()
    }

    pub fn mapped_regions(&self) -> Vec<[i64; 4]> {
        self.streaming
            .as_ref()
            .map(|s| s.stream.mapped_regions())
            .unwrap_or_default()
    }

    pub fn map_demands(&self) -> Result<Vec<[i64; 4]>, StreamError> {
        let Some(state) = &self.streaming else {
            return Ok(Vec::new());
        };
        state
            .stream
            .map_demands(state.snapshot.global_position(self.stream_anchor()))
    }

    pub fn mapped_source_parts(&self) -> usize {
        self.streaming
            .as_ref()
            .map_or(0, |s| s.stream.mapped_source_parts())
    }

    pub fn map_waiting(&self) -> bool {
        self.streaming.as_ref().is_some_and(|s| {
            s.stream.map_anchor().is_some()
                && s.pending_origin.is_some()
                && !s.stream.progress().complete
        })
    }

    /// Adjacent immutable source arrives without replacing the Session, moving
    /// the camera, or resetting designs, gameplay and construction history.
    pub fn append_map_region(&mut self, region: MapRegion) -> Result<bool, StreamError> {
        if self
            .streaming
            .as_ref()
            .is_none_or(|s| s.stream.map_anchor().is_none())
        {
            return Err(StreamError::InvalidSave(
                "Open a mapped world before appending geographic regions".into(),
            ));
        }
        if self.import_progress().is_some() {
            return Err(StreamError::InvalidSave(
                "Finish the active construction import before adding map data".into(),
            ));
        }
        self.capture_streamed_edits()?;
        self.streaming
            .as_mut()
            .expect("mapped session")
            .stream
            .publish_map_region(region)
    }

    /// Keep a body inside the last complete resident window while its next map
    /// region is pending. Its next simulation steps resume naturally after rebase.
    /// Orbit mode guards the subject, allowing the eye to stay outside the slab.
    pub(crate) fn constrain_map_camera(&mut self) -> bool {
        if self.map_anchor().is_none() || self.active_vehicle.is_some() {
            return false;
        }
        let anchor = if self.camera.mode == CameraMode::Orbit {
            self.camera.target
        } else {
            self.camera.position
        };
        let constrained = Vec3::new(
            anchor.x.clamp(8.0, 88.0),
            anchor.y,
            anchor.z.clamp(8.0, 88.0),
        );
        let delta = constrained - anchor;
        if delta.length_squared() <= 0.000_000_1 {
            return false;
        }
        self.camera.position += delta;
        self.camera.target += delta;
        self.status = "Waiting for nearby map terrain · movement continues when the area is ready. You can turn, build, or retry the source in Places.".into();
        true
    }
}

fn walking_spawn(world: &briccraft_world::World, x: f32, z: f32) -> [f32; 3] {
    // Rank complete, collision-free bodies by floor height, then proximity. A
    // low centre sample alone can put the circular footprint inside a wall or
    // its head beneath an adjoining overhang and force a first-frame roof jump.
    let mut controller = briccraft_character::CharacterController::default();
    let grid = crate::camera::WorldCollision(world);
    let eye_height = controller.config().eye_height;
    let mut best: Option<(f32, i32, Vec3)> = None;
    for dz in -8i32..=8 {
        for dx in -8i32..=8 {
            let px = (x + dx as f32 * 1.5).clamp(16.0, 80.0);
            let pz = (z + dz as f32 * 1.5).clamp(16.0, 80.0);
            let floor = world.surface_height(px, pz);
            let feet = Vec3::new(px, floor, pz);
            if !controller.can_occupy(feet, &grid) {
                continue;
            }
            let distance = dx * dx + dz * dz;
            if best.as_ref().is_none_or(|&(height, near, _)| {
                floor < height || (floor == height && distance < near)
            }) {
                best = Some((floor, distance, feet));
            }
        }
    }
    let feet = best.map_or_else(
        || {
            // A fully obstructed neighborhood still publishes a valid pose;
            // resolve its roof fallback before exposing the new session.
            controller
                .advance(
                    Vec3::new(x, world.surface_height(x, z), z),
                    briccraft_character::CharacterInput::default(),
                    0.0,
                    &grid,
                )
                .position
        },
        |(_, _, feet)| feet,
    );
    (feet + Vec3::Y * eye_height).to_array()
}

#[cfg(test)]
mod tests;
