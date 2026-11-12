//! Web Mercator display math shared with the tile host. Values are logical map pixels.
use crate::{MapView, UiLayout};

pub const MIN_MAP_ZOOM: u8 = 2;
pub const MAX_MAP_ZOOM: u8 = 19;
pub const MAP_TILE_SIZE: f64 = 256.0;
const RADIUS: f64 = 6_378_137.0;

pub fn map_panel_width(layout: UiLayout) -> f64 {
    (layout.width - 24.0).clamp(1.0, 1100.0)
}

pub fn map_wide(layout: UiLayout) -> bool {
    layout.width >= 900.0 && layout.height >= 540.0
}

/// This exact size is used for the native viewport and the host's visible-tile request.
pub fn map_view_size(layout: UiLayout) -> [f64; 2] {
    let width = (map_panel_width(layout) - 34.0).max(1.0);
    if map_wide(layout) {
        [
            (width - 316.0).max(1.0),
            (layout.height - 230.0).clamp(240.0, 640.0),
        ]
    } else {
        [width, (layout.height * 0.34).clamp(160.0, 300.0)]
    }
}

pub fn project(latitude: f64, longitude: f64, zoom: u8) -> [f64; 2] {
    let size = MAP_TILE_SIZE * 2_f64.powi(i32::from(zoom));
    let lat = latitude.clamp(-85.0, 85.0).to_radians();
    [
        (longitude + 180.0).rem_euclid(360.0) / 360.0 * size,
        (1.0 - lat.tan().asinh() / std::f64::consts::PI) * 0.5 * size,
    ]
}

pub fn unproject(point: [f64; 2], zoom: u8) -> [f64; 2] {
    let size = MAP_TILE_SIZE * 2_f64.powi(i32::from(zoom));
    [
        ((1.0 - point[1] / size * 2.0) * std::f64::consts::PI)
            .sinh()
            .atan()
            .to_degrees()
            .clamp(-85.0, 85.0),
        (point[0] / size * 360.0).rem_euclid(360.0) - 180.0,
    ]
}

fn wrapped_delta(value: f64, center: f64, zoom: u8) -> f64 {
    let size = MAP_TILE_SIZE * 2_f64.powi(i32::from(zoom));
    (value - center + size * 0.5).rem_euclid(size) - size * 0.5
}

pub fn screen_position(view: &MapView, size: [f64; 2], latitude: f64, longitude: f64) -> [f64; 2] {
    let center = project(view.latitude, view.longitude, view.zoom);
    let point = project(latitude, longitude, view.zoom);
    [
        size[0] * 0.5 + wrapped_delta(point[0], center[0], view.zoom),
        size[1] * 0.5 + point[1] - center[1],
    ]
}

pub fn pick_position(view: &MapView, size: [f64; 2], point: [f64; 2]) -> [f64; 2] {
    let center = project(view.latitude, view.longitude, view.zoom);
    unproject(
        [
            center[0] + point[0] - size[0] * 0.5,
            center[1] + point[1] - size[1] * 0.5,
        ],
        view.zoom,
    )
}

pub fn pan(view: &mut MapView, delta: [f64; 2]) {
    let center = project(view.latitude, view.longitude, view.zoom);
    let next = unproject([center[0] - delta[0], center[1] - delta[1]], view.zoom);
    view.latitude = next[0];
    view.longitude = next[1];
}

pub fn zoom_at(view: &mut MapView, size: [f64; 2], point: [f64; 2], delta: i8) {
    let zoom = (i16::from(view.zoom) + i16::from(delta))
        .clamp(i16::from(MIN_MAP_ZOOM), i16::from(MAX_MAP_ZOOM)) as u8;
    if zoom == view.zoom {
        return;
    }
    let anchor = pick_position(view, size, point);
    let projected = project(anchor[0], anchor[1], zoom);
    let center = unproject(
        [
            projected[0] - point[0] + size[0] * 0.5,
            projected[1] - point[1] + size[1] * 0.5,
        ],
        zoom,
    );
    view.latitude = center[0];
    view.longitude = center[1];
    view.zoom = zoom;
}

pub fn meters_per_pixel(latitude: f64, zoom: u8) -> f64 {
    latitude.clamp(-85.0, 85.0).to_radians().cos() * std::f64::consts::TAU * RADIUS
        / (MAP_TILE_SIZE * 2_f64.powi(i32::from(zoom)))
}

pub fn tile_position(view: &MapView, size: [f64; 2], x: u32, y: u32) -> [f64; 2] {
    let center = project(view.latitude, view.longitude, view.zoom);
    [
        size[0] * 0.5 + wrapped_delta((f64::from(x) + 0.5) * MAP_TILE_SIZE, center[0], view.zoom)
            - MAP_TILE_SIZE * 0.5,
        size[1] * 0.5 + f64::from(y) * MAP_TILE_SIZE - center[1],
    ]
}

/// Only tiles intersecting the actual viewport; no neighbouring/offline prefetch.
pub fn visible_map_tiles(view: &MapView, size: [f64; 2]) -> Vec<(u8, u32, u32)> {
    let zoom = view.zoom.clamp(MIN_MAP_ZOOM, MAX_MAP_ZOOM);
    let count = 1_i64 << zoom;
    let center = project(view.latitude, view.longitude, zoom);
    let x0 = ((center[0] - size[0] * 0.5) / MAP_TILE_SIZE).floor() as i64;
    let x1 = ((center[0] + size[0] * 0.5 - 0.0001) / MAP_TILE_SIZE).floor() as i64;
    let y0 = ((center[1] - size[1] * 0.5) / MAP_TILE_SIZE).floor() as i64;
    let y1 = ((center[1] + size[1] * 0.5 - 0.0001) / MAP_TILE_SIZE).floor() as i64;
    let mut tiles = Vec::new();
    for y in y0.max(0)..=y1.min(count - 1) {
        for x in x0..=x1 {
            let key = (zoom, x.rem_euclid(count) as u32, y as u32);
            if !tiles.contains(&key) {
                tiles.push(key);
            }
        }
    }
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mercator_coordinates_round_trip_real_places() {
        for (latitude, longitude) in [
            (52.516275, 13.377704),
            (51.5, -0.124625),
            (-33.86, 151.21),
            (80.0, 179.999),
            (-80.0, -179.999),
        ] {
            for zoom in [2, 10, 19] {
                let result = unproject(project(latitude, longitude, zoom), zoom);
                assert!((result[0] - latitude).abs() < 1e-8);
                assert!((result[1] - longitude).abs() < 1e-8);
            }
        }
    }

    #[test]
    fn geographic_pick_and_anchor_zoom_round_trip_across_date_line() {
        for latitude in [-80.0, 0.0, 52.5, 80.0] {
            let mut view = MapView {
                latitude,
                longitude: 179.999,
                zoom: 14,
                ..Default::default()
            };
            let point = [480.0, 180.0];
            let size = [640.0, 360.0];
            let selected = pick_position(&view, size, point);
            zoom_at(&mut view, size, point, 2);
            let projected = screen_position(&view, size, selected[0], selected[1]);
            assert!((projected[0] - point[0]).abs() < 1e-7);
            assert!((projected[1] - point[1]).abs() < 1e-7);
        }
    }

    #[test]
    fn visible_tiles_are_bounded_and_reach_view_edges() {
        for layout in [
            UiLayout::default(),
            UiLayout::new([390.0, 844.0], [0.0; 4], [0.0; 2], true),
        ] {
            let size = map_view_size(layout);
            let view = MapView::default();
            let tiles = visible_map_tiles(&view, size);
            assert!(!tiles.is_empty() && tiles.len() <= 20);
            for (_, x, y) in tiles {
                let p = tile_position(&view, size, x, y);
                assert!(p[0] < size[0] && p[0] + 256.0 > 0.0);
                assert!(p[1] < size[1] && p[1] + 256.0 > 0.0);
            }
        }
    }

    #[test]
    fn panning_does_not_change_selected_build_area() {
        let mut view = MapView::default();
        let selected = [view.selected_latitude, view.selected_longitude];
        pan(&mut view, [320.0, -250.0]);
        assert_eq!(selected, [view.selected_latitude, view.selected_longitude]);
        assert!(view.longitude < selected[1] && view.latitude < selected[0]);
    }
}
