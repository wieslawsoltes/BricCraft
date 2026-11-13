//! Small platform-independent value model for geographic orientation.
use crate::MapView;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapPlayer {
    pub latitude: f64,
    pub longitude: f64,
    pub bearing_degrees: f64,
    pub distance_from_start_m: f64,
}

impl MapPlayer {
    pub fn heading(self) -> String {
        let bearing = self.bearing_degrees.rem_euclid(360.0);
        let direction = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"]
            [((bearing + 22.5) / 45.0).floor() as usize % 8];
        format!("{direction} · {:03}°", bearing.round() as u16 % 360)
    }

    pub fn coordinates(self) -> String {
        format!("{:.5}, {:.5}", self.latitude, self.longitude)
    }

    pub fn distance(self) -> String {
        if self.distance_from_start_m >= 1000.0 {
            format!("{:.1} km", self.distance_from_start_m / 1000.0)
        } else {
            format!("{:.0} m", self.distance_from_start_m)
        }
    }

    /// Center the display only. A player location is never a new build selection.
    pub fn center_view(self, view: &mut MapView) {
        view.latitude = self.latitude;
        view.longitude = self.longitude;
        view.zoom = view.zoom.max(16);
    }

    /// A geographic heading arrow in a fixed 48-pixel marker box.
    pub fn arrow(self) -> [[f64; 2]; 3] {
        let bearing = self.bearing_degrees.to_radians();
        let point =
            |angle: f64, radius: f64| [24.0 + angle.sin() * radius, 24.0 - angle.cos() * radius];
        [
            point(bearing, 20.0),
            point(bearing + 2.45, 9.0),
            point(bearing - 2.45, 9.0),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(bearing: f64) -> MapPlayer {
        MapPlayer {
            latitude: 50.0916,
            longitude: 18.2199,
            bearing_degrees: bearing,
            distance_from_start_m: 145.0,
        }
    }

    #[test]
    fn locating_player_preserves_selected_area_and_generation_options() {
        let mut view = MapView::default();
        view.zoom = 10;
        let before = view.clone();
        let player = player(90.0);
        player.center_view(&mut view);
        assert_eq!(
            [view.latitude, view.longitude],
            [player.latitude, player.longitude]
        );
        assert_eq!(view.zoom, 16);
        view.latitude = before.latitude;
        view.longitude = before.longitude;
        view.zoom = before.zoom;
        assert_eq!(view, before);
    }

    #[test]
    fn heading_and_arrow_share_clockwise_geographic_north() {
        for (bearing, label, tip) in [
            (0.0, "N · 000°", [24.0, 4.0]),
            (90.0, "E · 090°", [44.0, 24.0]),
            (180.0, "S · 180°", [24.0, 44.0]),
            (270.0, "W · 270°", [4.0, 24.0]),
            (359.99, "N · 000°", [23.996509, 4.0]),
        ] {
            let player = player(bearing);
            assert_eq!(player.heading(), label);
            let actual = player.arrow()[0];
            assert!((actual[0] - tip[0]).abs() < 0.001);
            assert!((actual[1] - tip[1]).abs() < 0.001);
        }
    }

    #[test]
    fn distance_units_remain_compact_and_coordinates_keep_hemisphere_signs() {
        let mut player = player(45.0);
        assert_eq!(player.distance(), "145 m");
        player.distance_from_start_m = 1820.0;
        assert_eq!(player.distance(), "1.8 km");
        player.latitude = -33.86;
        player.longitude = -151.21;
        assert_eq!(player.coordinates(), "-33.86000, -151.21000");
    }
}
