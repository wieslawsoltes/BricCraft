use crate::AssemblyAssessment;
use briccraft_parts::{part, Shape};
use briccraft_world::Instance;

fn center(instance: &Instance) -> [f32; 3] {
    let p = instance.world_position();
    let s = instance.size();
    [
        p[0] + s[0] as f32 * 0.5,
        p[1] + s[1] as f32 * 0.2,
        p[2] + s[2] as f32 * 0.5,
    ]
}
fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>().sqrt()
}
fn teeth(id: u16) -> Option<u16> {
    match id {
        90 => Some(8),
        91 => Some(16),
        92 => Some(24),
        93 => Some(40),
        _ => None,
    }
}

pub(crate) fn measure_drive(instances: &[Instance], assessment: &mut AssemblyAssessment) {
    let heading_anchor = instances
        .iter()
        .find(|instance| matches!(instance.part_id, 120 | 121 | 127 | 135))
        .or_else(|| {
            instances.iter().max_by_key(|instance| {
                let size = instance.size();
                u32::from(size[0]) * u32::from(size[2])
            })
        });
    let quarter = heading_anchor.map_or(0, |instance| instance.rotation);
    assessment.forward_yaw = quarter as f32 * std::f32::consts::FRAC_PI_2;
    let wheels: Vec<_> = instances
        .iter()
        .filter(|instance| part(instance.part_id).is_some_and(|part| part.shape == Shape::Wheel))
        .collect();
    if !wheels.is_empty() {
        assessment.wheel_radius = wheels
            .iter()
            .map(|wheel| wheel.size()[1] as f32 * 0.2)
            .sum::<f32>()
            / wheels.len() as f32;
        let extent = |axis: usize| {
            let min = wheels
                .iter()
                .map(|wheel| center(wheel)[axis])
                .fold(f32::INFINITY, f32::min);
            let max = wheels
                .iter()
                .map(|wheel| center(wheel)[axis])
                .fold(f32::NEG_INFINITY, f32::max);
            (max - min).max(1.0)
        };
        assessment.wheelbase = extent(if quarter & 1 == 0 { 2 } else { 0 });
        assessment.track = extent(if quarter & 1 == 0 { 0 } else { 2 });
    }
    let motors: Vec<_> = instances
        .iter()
        .filter(|instance| matches!(instance.part_id, 140 | 141 | 144))
        .collect();
    let gears: Vec<_> = instances
        .iter()
        .filter_map(|instance| teeth(instance.part_id).map(|teeth| (instance, teeth)))
        .collect();
    // Infer only a directly touching external gear pair. Arbitrary trains are represented explicitly
    // by GearTrain; merely adding an unrelated gear to a vehicle never changes its reduction.
    for (i, (a, ta)) in gears.iter().enumerate() {
        for (b, tb) in gears.iter().skip(i + 1) {
            if a.rotation % 2 != b.rotation % 2 {
                continue;
            }
            let ca = center(a);
            let cb = center(b);
            let axle = if a.rotation % 2 == 0 { 0 } else { 2 };
            let separation = (0..3)
                .filter(|axis| *axis != axle)
                .map(|axis| (ca[axis] - cb[axis]).powi(2))
                .sum::<f32>()
                .sqrt();
            let pitch = (*ta as f32 + *tb as f32) / 16.0;
            if (ca[axle] - cb[axle]).abs() > 0.55 || (separation - pitch).abs() > 0.22 {
                continue;
            }
            let da = motors
                .iter()
                .map(|motor| distance(ca, center(motor)))
                .fold(f32::INFINITY, f32::min);
            let db = motors
                .iter()
                .map(|motor| distance(cb, center(motor)))
                .fold(f32::INFINITY, f32::min);
            if !da.is_finite() {
                continue;
            }
            assessment.drive_ratio = if da <= db {
                *tb as f32 / *ta as f32
            } else {
                *ta as f32 / *tb as f32
            };
            return;
        }
    }
}
