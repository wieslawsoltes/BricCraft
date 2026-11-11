//! Versioned session persistence. A failed import leaves the current session intact.
use crate::{CameraRig, Session};
use briccraft_world::{BuildError, World};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Save {
    version: u32,
    world: serde_json::Value,
    camera: CameraRig,
    seed: u64,
    selected_part: u16,
    color: u8,
    rotation: u8,
    daylight: f32,
    quality: u8,
}

impl Session {
    pub fn to_json(&self) -> Result<String, BuildError> {
        if self.active_vehicle.is_some() {
            return Err(BuildError::InvalidSave(
                "Park the active vehicle before saving.".into(),
            ));
        }
        let world = serde_json::from_str(&self.world.to_json()?)
            .map_err(|e| BuildError::InvalidSave(e.to_string()))?;
        serde_json::to_string(&Save {
            version: 1,
            world,
            camera: self.camera.clone(),
            seed: self.seed,
            selected_part: self.selected_part,
            color: self.color,
            rotation: self.rotation,
            daylight: self.daylight,
            quality: self.quality,
        })
        .map_err(|e| BuildError::InvalidSave(e.to_string()))
    }
    pub fn import_json(&mut self, json: &str) -> Result<(), BuildError> {
        if json.len() > 16 * 1024 * 1024 {
            return Err(BuildError::InvalidSave(
                "The world file exceeds 16 MB.".into(),
            ));
        }
        let save: Save =
            serde_json::from_str(json).map_err(|e| BuildError::InvalidSave(e.to_string()))?;
        if save.version != 1 {
            return Err(BuildError::InvalidSave(format!(
                "Unsupported session version {}.",
                save.version
            )));
        }
        if !save.camera.position.is_finite()
            || !save.camera.target.is_finite()
            || !save.camera.yaw.is_finite()
            || !save.camera.pitch.is_finite()
            || !(0.1..3.0).contains(&save.camera.vertical_fov)
            || !save.camera.distance.is_finite()
            || save.camera.distance <= 0.0
            || briccraft_parts::part(save.selected_part).is_none()
            || usize::from(save.color) >= briccraft_parts::PALETTE.len()
            || save.rotation >= 4
            || !save.daylight.is_finite()
            || !(0.0..=1.0).contains(&save.daylight)
            || save.quality > 2
        {
            return Err(BuildError::InvalidSave(
                "Invalid camera or construction settings.".into(),
            ));
        }
        let world = World::from_json(&save.world.to_string())?;
        self.world = world;
        self.camera = save.camera;
        self.seed = save.seed;
        self.selected_part = save.selected_part;
        self.color = save.color;
        self.rotation = save.rotation;
        self.daylight = save.daylight;
        self.quality = save.quality;
        self.history.clear();
        self.active_vehicle = None;
        self.last_blueprint.clear();
        self.input.clear();
        self.status = "World loaded. Your construction is ready.".into();
        self.update_pick();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrupt_import_does_not_change_live_world() {
        let mut session = Session::new(42);
        let count = session.world.len();
        let revision = session.world.revision();
        assert!(session.import_json("{\"version\":999}").is_err());
        assert_eq!(session.world.len(), count);
        assert_eq!(session.world.revision(), revision);
    }
    #[test]
    fn session_round_trip_preserves_construction_and_camera() {
        let mut session = Session::new(42);
        session.camera.orbit(35.0, 20.0);
        let json = session.to_json().unwrap();
        let mut loaded = Session::new(7);
        loaded.import_json(&json).unwrap();
        assert_eq!(loaded.world.len(), session.world.len());
        assert_eq!(loaded.seed, 42);
        assert!((loaded.camera.position - session.camera.position).length() < 1e-4);
    }
}
