#[derive(Debug, Clone, Copy)]
pub struct RailSample {
    pub position: [f32; 3],
    pub tangent: [f32; 3],
    pub yaw: f32,
}

#[derive(Debug, Clone)]
pub struct RailPath {
    points: Vec<[f32; 3]>,
    cumulative: Vec<f32>,
    length: f32,
    closed: bool,
}
impl RailPath {
    pub fn new(points: Vec<[f32; 3]>, closed: bool) -> Option<Self> {
        if points.len() < 2 || points.iter().flatten().any(|v| !v.is_finite()) {
            return None;
        }
        let mut cumulative = vec![0.0];
        let segments = points.len() - usize::from(!closed);
        let mut length = 0.0;
        for i in 0..segments {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            let d = (0..3)
                .map(|axis| (a[axis] - b[axis]).powi(2))
                .sum::<f32>()
                .sqrt();
            if d < 1e-5 {
                return None;
            }
            length += d;
            cumulative.push(length);
        }
        Some(Self {
            points,
            cumulative,
            length,
            closed,
        })
    }
    pub fn coastal() -> Self {
        Self::new(
            vec![
                [20.0, 8.0, 20.0],
                [74.0, 8.0, 20.0],
                [74.0, 8.0, 74.0],
                [20.0, 8.0, 74.0],
            ],
            true,
        )
        .expect("valid coastal track")
    }
    pub fn length(&self) -> f32 {
        self.length
    }
    pub fn sample(&self, distance: f32) -> RailSample {
        let distance = if self.closed {
            distance.rem_euclid(self.length)
        } else {
            distance.clamp(0.0, self.length - 1e-5)
        };
        let index = self
            .cumulative
            .partition_point(|value| *value <= distance)
            .saturating_sub(1)
            .min(self.cumulative.len() - 2);
        let a = self.points[index];
        let b = self.points[(index + 1) % self.points.len()];
        let len = self.cumulative[index + 1] - self.cumulative[index];
        let t = (distance - self.cumulative[index]) / len;
        let tangent = std::array::from_fn(|axis| (b[axis] - a[axis]) / len);
        RailSample {
            position: std::array::from_fn(|axis| a[axis] + (b[axis] - a[axis]) * t),
            tangent,
            yaw: tangent[0].atan2(tangent[2]),
        }
    }
    pub fn closest_distance(&self, position: [f32; 3]) -> f32 {
        let mut best = (f32::INFINITY, 0.0);
        for i in 0..self.cumulative.len() - 1 {
            let a = self.points[i];
            let b = self.points[(i + 1) % self.points.len()];
            let d: [f32; 3] = std::array::from_fn(|axis| b[axis] - a[axis]);
            let len2 = d.into_iter().map(|v| v * v).sum::<f32>();
            let t = ((0..3)
                .map(|axis| (position[axis] - a[axis]) * d[axis])
                .sum::<f32>()
                / len2)
                .clamp(0.0, 1.0);
            let distance = (0..3)
                .map(|axis| (position[axis] - a[axis] - t * d[axis]).powi(2))
                .sum::<f32>();
            if distance < best.0 {
                best = (distance, self.cumulative[i] + t * len2.sqrt());
            }
        }
        best.1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn closed_path_wraps_without_losing_distance() {
        let rail = RailPath::coastal();
        let a = rail.sample(5.0);
        let b = rail.sample(rail.length() + 5.0);
        assert_eq!(a.position, b.position);
        assert!((rail.closest_distance(a.position) - 5.0).abs() < 0.001);
    }
}
