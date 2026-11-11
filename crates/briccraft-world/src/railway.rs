//! Shared centerline for the generated track and train simulation.
//! The two rails lie 3.5 studs either side of this rounded-rectangle route.

pub const COASTAL_RAIL_HEIGHT: f32 = 8.0;
pub const COASTAL_RAIL_GAUGE: f32 = 7.0;
pub const COASTAL_RAIL_RADIUS: f32 = 6.0;
const STRAIGHT: f32 = 42.0;
const ARC: f32 = COASTAL_RAIL_RADIUS * std::f32::consts::FRAC_PI_2;
const LEG: f32 = STRAIGHT + ARC;
pub const COASTAL_RAIL_LENGTH: f32 = 4.0 * LEG;

#[derive(Debug, Clone, Copy)]
pub struct CoastalRailSample {
    pub position: [f32; 3],
    pub tangent: [f32; 3],
    pub yaw: f32,
}

fn turn(p: [f32; 2], quarter: usize) -> [f32; 2] {
    match quarter & 3 {
        0 => p,
        1 => [-p[1], p[0]],
        2 => [-p[0], -p[1]],
        _ => [p[1], -p[0]],
    }
}

pub fn coastal_rail_sample(distance: f32) -> CoastalRailSample {
    let distance = if distance.is_finite() {
        distance.rem_euclid(COASTAL_RAIL_LENGTH)
    } else {
        0.0
    };
    let quarter = (distance / LEG).floor() as usize;
    let local = distance - quarter as f32 * LEG;
    let (point, tangent) = if local <= STRAIGHT {
        ([26.0 + local, 20.0], [1.0, 0.0])
    } else {
        let angle = (local - STRAIGHT) / COASTAL_RAIL_RADIUS - std::f32::consts::FRAC_PI_2;
        let (sin, cos) = angle.sin_cos();
        (
            [
                68.0 + COASTAL_RAIL_RADIUS * cos,
                26.0 + COASTAL_RAIL_RADIUS * sin,
            ],
            [-sin, cos],
        )
    };
    let p = turn([point[0] - 47.0, point[1] - 47.0], quarter);
    let t = turn(tangent, quarter);
    CoastalRailSample {
        position: [p[0] + 47.0, COASTAL_RAIL_HEIGHT, p[1] + 47.0],
        tangent: [t[0], 0.0, t[1]],
        yaw: t[0].atan2(t[1]),
    }
}

/// Exact nearest point on four straight spans and four quarter circles; no sampled-path drift.
pub fn coastal_rail_closest_distance(position: [f32; 3]) -> f32 {
    if position.iter().any(|v| !v.is_finite()) {
        return 0.0;
    }
    let mut best = (f32::INFINITY, 0.0);
    for quarter in 0..4 {
        let p = turn([position[0] - 47.0, position[2] - 47.0], (4 - quarter) & 3);
        let p = [p[0] + 47.0, p[1] + 47.0];
        let x = p[0].clamp(26.0, 68.0);
        let straight_distance = (p[0] - x).powi(2) + (p[1] - 20.0).powi(2);
        if straight_distance < best.0 {
            best = (straight_distance, quarter as f32 * LEG + x - 26.0);
        }
        let angle = (p[1] - 26.0)
            .atan2(p[0] - 68.0)
            .clamp(-std::f32::consts::FRAC_PI_2, 0.0);
        let nearest = [
            68.0 + COASTAL_RAIL_RADIUS * angle.cos(),
            26.0 + COASTAL_RAIL_RADIUS * angle.sin(),
        ];
        let arc_distance = (p[0] - nearest[0]).powi(2) + (p[1] - nearest[1]).powi(2);
        if arc_distance < best.0 {
            best = (
                arc_distance,
                quarter as f32 * LEG
                    + STRAIGHT
                    + (angle + std::f32::consts::FRAC_PI_2) * COASTAL_RAIL_RADIUS,
            );
        }
    }
    best.1.rem_euclid(COASTAL_RAIL_LENGTH)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounded_track_is_closed_and_has_continuous_tangents() {
        for boundary in [
            STRAIGHT,
            LEG,
            LEG + STRAIGHT,
            2.0 * LEG,
            2.0 * LEG + STRAIGHT,
            3.0 * LEG,
            3.0 * LEG + STRAIGHT,
            COASTAL_RAIL_LENGTH,
        ] {
            let a = coastal_rail_sample(boundary - 0.001);
            let b = coastal_rail_sample(boundary + 0.001);
            assert!(
                (0..3)
                    .map(|i| (a.position[i] - b.position[i]).powi(2))
                    .sum::<f32>()
                    .sqrt()
                    < 0.003
            );
            assert!(
                (0..3)
                    .map(|i| (a.tangent[i] - b.tangent[i]).powi(2))
                    .sum::<f32>()
                    .sqrt()
                    < 0.001
            );
        }
        for distance in [0.0, 3.0, 44.0, 49.0, 70.0, 124.0, 191.0] {
            let sample = coastal_rail_sample(distance);
            assert!((coastal_rail_closest_distance(sample.position) - distance).abs() < 0.001);
        }
        let east = coastal_rail_sample(coastal_rail_closest_distance([74.0, 8.0, 44.0]));
        assert!((east.position[0] - 74.0).abs() < 0.0001);
        assert!((east.position[1] - 8.0).abs() < 0.0001);
        assert!((east.position[2] - 44.0).abs() < 0.0001);
        assert_eq!(east.yaw, 0.0);
    }
}
