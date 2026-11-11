//! Versioned session persistence. Candidate worlds are validated before replacing the live session.
use crate::{CameraRig, Session, StreamSession, WorldInfo};
use briccraft_stream::{StreamWorld, WindowOrigin};
use briccraft_world::{BuildError, World};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Save {
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    world: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stream: Option<serde_json::Value>,
    #[serde(default)]
    origin: WindowOrigin,
    #[serde(default)]
    world_info: Option<WorldInfo>,
    #[serde(default)]
    gameplay: briccraft_play::Gameplay,
    camera: CameraRig,
    seed: u64,
    selected_part: u16,
    color: u8,
    rotation: u8,
    daylight: f32,
    quality: u8,
}

fn invalid(error: impl std::fmt::Display) -> BuildError {
    BuildError::InvalidSave(error.to_string())
}

impl Session {
    pub fn to_json(&self) -> Result<String, BuildError> {
        validate_info(&self.world_info)?;
        self.gameplay.validate().map_err(invalid)?;
        if self.active_vehicle.is_some() {
            return Err(invalid("Park the active vehicle before saving."));
        }
        let (world, stream) = if let Some(state) = &self.streaming {
            // Capture explicit caller edits made through the public World API as well. Copies keep
            // this read-only operation atomic and leave live global construction history untouched.
            let mut stream = state.stream.clone();
            let mut snapshot = state.snapshot.clone();
            stream
                .capture_edits(&mut snapshot, &self.world)
                .map_err(invalid)?;
            (
                None,
                Some(serde_json::from_str(&stream.to_json().map_err(invalid)?).map_err(invalid)?),
            )
        } else {
            (
                Some(serde_json::from_str(&self.world.to_json()?).map_err(invalid)?),
                None,
            )
        };
        serde_json::to_string(&Save {
            version: 2,
            world,
            stream,
            origin: self.world_origin(),
            world_info: Some(self.world_info.clone()),
            gameplay: self.gameplay.clone(),
            camera: self.camera.clone(),
            seed: self.seed,
            selected_part: self.selected_part,
            color: self.color,
            rotation: self.rotation,
            daylight: self.daylight,
            quality: self.quality,
        })
        .map_err(invalid)
    }

    pub fn import_json(&mut self, json: &str) -> Result<(), BuildError> {
        if json.len() > 64 * 1024 * 1024 {
            return Err(invalid("The world file exceeds 64 MiB."));
        }
        let save: Save = serde_json::from_str(json).map_err(invalid)?;
        if save.version != 1 && save.version != 2 {
            return Err(invalid(format!(
                "Unsupported session version {}.",
                save.version
            )));
        }
        validate_settings(&save)?;
        let (world, streaming, mut info) = if let Some(stream) = save.stream {
            if save.version != 2 || save.world.is_some() {
                return Err(invalid(
                    "A streamed save must contain exactly one version 2 world representation.",
                ));
            }
            let mut stream = StreamWorld::from_json(&stream.to_string()).map_err(invalid)?;
            if stream.seed() != save.seed || stream.origin() != save.origin {
                return Err(invalid(
                    "Stream seed/origin does not match its session envelope.",
                ));
            }
            let info = save
                .world_info
                .unwrap_or_else(|| WorldInfo::streamed(save.seed, stream.preset()));
            if info.preset != Some(stream.preset()) {
                return Err(invalid("World metadata does not match its terrain preset."));
            }
            let snapshot = stream.prepare_window(save.origin).map_err(invalid)?;
            (
                snapshot.world.clone(),
                Some(StreamSession::new(stream, snapshot)),
                info,
            )
        } else {
            let value = save
                .world
                .ok_or_else(|| invalid("The save contains no world."))?;
            let world = World::from_json(&value.to_string())?;
            if world.seed != save.seed {
                return Err(invalid("World seed does not match its session envelope."));
            }
            let info = save
                .world_info
                .unwrap_or_else(|| WorldInfo::classic(save.seed));
            if info.preset.is_some() {
                return Err(invalid(
                    "Classic island metadata cannot specify a streamed terrain preset.",
                ));
            }
            (world, None, info)
        };
        if info.id.is_empty() {
            info.id = format!("loaded-{}", save.seed);
        }
        validate_info(&info)?;
        self.world = world;
        self.streaming = streaming;
        self.world_info = info;
        self.gameplay = save.gameplay;
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
        self.accumulator = 0.0;
        self.scene_generation = self.scene_generation.wrapping_add(1);
        self.status = format!(
            "{} loaded. Your construction is ready.",
            self.world_info.name
        );
        self.update_pick();
        Ok(())
    }
}

fn validate_settings(save: &Save) -> Result<(), BuildError> {
    save.gameplay.validate().map_err(invalid)?;
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
        return Err(invalid("Invalid camera or construction settings."));
    }
    Ok(())
}

fn validate_info(info: &WorldInfo) -> Result<(), BuildError> {
    if info.id.len() > 256
        || info.name.len() > 1_024
        || info.attributions.len() > 64
        || info
            .import_report
            .as_ref()
            .is_some_and(|report| report.len() > 1_048_576)
        || info.attributions.iter().any(|source| {
            source.label.len() > 4_096
                || source.license.len() > 4_096
                || source.url.len() > 8_192
                || (!source.url.is_empty()
                    && !source.url.starts_with("https://")
                    && !source.url.starts_with("http://"))
        })
    {
        return Err(invalid("Invalid or oversized world/source metadata."));
    }
    if let Some(report) = &info.import_report {
        let value: serde_json::Value = serde_json::from_str(report)
            .map_err(|_| invalid("The source import report must be valid JSON."))?;
        if !value.is_object() {
            return Err(invalid("The source import report must be a JSON object."));
        }
    }
    Ok(())
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
    #[test]
    fn legacy_version_one_is_still_readable() {
        let session = Session::new(42);
        let mut value: serde_json::Value =
            serde_json::from_str(&session.to_json().unwrap()).unwrap();
        value["version"] = 1.into();
        let object = value.as_object_mut().unwrap();
        object.remove("world_info");
        object.remove("origin");
        let mut loaded = Session::new(7);
        loaded.import_json(&value.to_string()).unwrap();
        assert_eq!(loaded.world_info.name, "Sunlit Archipelago");
        assert!(!loaded.is_streamed());
        assert_eq!(loaded.world.len(), session.world.len());
    }

    #[test]
    fn public_metadata_cannot_serialize_a_save_that_metadata_validation_rejects() {
        let mut session = Session::new(42);
        for report in ["not JSON", "5", "[]", "\"plain text\""] {
            session.world_info.import_report = Some(report.into());
            assert!(
                session.to_json().is_err(),
                "accepted malformed report {report}"
            );
        }
        session.world_info.import_report =
            Some(format!("{{\"payload\":\"{}\"}}", "x".repeat(1_048_576)));
        assert!(session.to_json().is_err());
        session.world_info.import_report = Some("{\"source\":\"fixture\"}".into());
        session.world_info.attributions = vec![crate::GameAttribution {
            label: "Unsafe URL fixture".into(),
            url: "javascript:alert(1)".into(),
            license: "CC0".into(),
        }];
        assert!(session.to_json().is_err());
        session.world_info.attributions.clear();
        session.world_info.name = "x".repeat(1_025);
        assert!(session.to_json().is_err());
    }

    #[test]
    fn malformed_loaded_report_keeps_current_world_and_metadata_intact() {
        let mut session = Session::new(42);
        let original_info = session.world_info.clone();
        let original_revision = session.world.revision();
        let mut source: serde_json::Value =
            serde_json::from_str(&session.to_json().unwrap()).unwrap();
        source["world_info"]["import_report"] = "5".into();
        assert!(session.import_json(&source.to_string()).is_err());
        assert_eq!(session.world_info, original_info);
        assert_eq!(session.world.revision(), original_revision);
    }
}
