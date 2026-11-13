//! Geographic navigation samples use the same focus and fixed projection as streaming.
use crate::Session;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MappedLocation {
    pub latitude: f64,
    pub longitude: f64,
    /// Clockwise from geographic north, in [0, 360).
    pub bearing_degrees: f64,
    /// Horizontal displacement from the selected starting pin, not route length.
    pub distance_from_start_m: f64,
}

impl Session {
    /// Locate the walking body, active vehicle, or orbit subject. Using the eye
    /// would move an overview marker while zooming and put a driver behind the car.
    pub fn mapped_location(&self) -> Option<MappedLocation> {
        let anchor = self.map_anchor()?;
        let position = self.world_origin().global_position(self.stream_anchor());
        let [latitude, longitude] = anchor.unproject(position[0], position[2]).ok()?;
        let yaw = self
            .active_vehicle
            .as_ref()
            .map_or(self.camera.yaw, |active| active.vehicle.yaw);
        if !yaw.is_finite() {
            return None;
        }
        // Game +X is east, +Z is south; its yaw zero looks south.
        let bearing_degrees = (180.0 - f64::from(yaw).to_degrees()).rem_euclid(360.0);
        let east = (position[0] - f64::from(anchor.origin[0])) * anchor.meters_per_stud;
        let south = (position[2] - f64::from(anchor.origin[2])) * anchor.meters_per_stud;
        Some(MappedLocation {
            latitude,
            longitude,
            bearing_degrees,
            distance_from_start_m: east.hypot(south),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CameraMode, MapAnchor, MapRegion};
    use glam::Vec3;

    fn session() -> Session {
        Session::new_mapped(
            42,
            MapAnchor {
                latitude: 52.516275,
                longitude: 13.377704,
                meters_per_stud: 2.0,
                origin: [64, 0, 64],
                reference_elevation_m: 0.0,
                meters_per_plate: 0.8,
                region_size: 128,
                terrain: false,
            },
            MapRegion {
                bounds: [0, 0, 128, 128],
                placements: Vec::new(),
            },
            "Navigation".into(),
            Vec::new(),
            true,
        )
        .unwrap()
    }

    #[test]
    fn north_east_south_west_follow_geographic_axes_and_wrapped_yaw() {
        let mut session = session();
        for (yaw, bearing) in [
            (0.0, 180.0),
            (std::f32::consts::FRAC_PI_2, 90.0),
            (std::f32::consts::PI, 0.0),
            (-std::f32::consts::FRAC_PI_2, 270.0),
            (std::f32::consts::TAU, 180.0),
        ] {
            session.camera.yaw = yaw;
            let actual = session.mapped_location().unwrap().bearing_degrees;
            let difference = (actual - bearing + 180.0).rem_euclid(360.0) - 180.0;
            assert!(difference.abs() < 0.001, "{actual} != {bearing}");
            assert!((0.0..360.0).contains(&actual));
        }
    }

    #[test]
    fn orbit_location_uses_subject_and_survives_resident_rebase() {
        let mut session = session();
        let origin = session.world_origin();
        session.camera.mode = CameraMode::Orbit;
        session.camera.target = Vec3::new(80.0 - origin.x as f32, 3.0, 64.0 - origin.z as f32);
        session.camera.position = session.camera.target + Vec3::new(-30.0, 25.0, -30.0);
        let location = session.mapped_location().unwrap();
        assert!((location.distance_from_start_m - 32.0).abs() < 1e-7);
        assert!((location.latitude - 52.516275).abs() < 1e-9);
        assert!(location.longitude > 13.377704);
        session.camera.position += Vec3::new(20.0, 10.0, 20.0);
        assert_eq!(session.mapped_location(), Some(location));
        session.camera.target.x += 16.0;
        let before = session.mapped_location();
        session
            .append_map_region(MapRegion {
                bounds: [128, 0, 256, 128],
                placements: Vec::new(),
            })
            .unwrap();
        for _ in 0..40 {
            session.advance_streaming();
        }
        assert_ne!(session.world_origin(), origin);
        assert_eq!(session.mapped_location(), before);
    }

    #[test]
    fn unmapped_world_and_nonfinite_heading_have_no_geographic_location() {
        assert!(Session::new(42).mapped_location().is_none());
        let mut session = session();
        session.camera.yaw = f32::NAN;
        assert!(session.mapped_location().is_none());
    }

    #[test]
    fn vehicle_location_tracks_the_chassis_independently_of_its_follow_camera() {
        let mut session = session();
        session.last_blueprint = briccraft_world::Blueprint::Rover
            .placements([40, 1, 40], 0)
            .into_iter()
            .map(|placement| session.world.place(placement).unwrap())
            .collect();
        session.activate_vehicle("car");
        let vehicle = &mut session.active_vehicle.as_mut().unwrap().vehicle;
        vehicle.yaw = std::f32::consts::FRAC_PI_2;
        let position = vehicle.position;
        let global = session.world_origin().global_position(position);
        let expected = session
            .map_anchor()
            .unwrap()
            .unproject(global[0], global[2])
            .unwrap();
        session.camera.position = Vec3::new(-1000.0, 25.0, 1000.0);
        session.camera.target = Vec3::ZERO;
        session.camera.yaw = 0.0;
        let location = session.mapped_location().unwrap();
        assert_eq!([location.latitude, location.longitude], expected);
        assert!((location.bearing_degrees - 90.0).abs() < 0.001);
    }
}
