use crate::VehicleKind;
use briccraft_parts::{Shape, part};
use briccraft_world::Instance;

#[derive(Debug, Clone)]
pub struct AssemblyAssessment {
    pub parts: usize,
    pub mass_kg: f32,
    pub wheels: usize,
    pub motors: usize,
    pub wings: usize,
    pub gears: usize,
    pub has_power: bool,
    pub suggested_vehicle: VehicleKind,
    /// Face-contact groups of the conservative part envelopes.
    pub connected_components: usize,
    pub drive_ready: bool,
    pub reasons: Vec<String>,
}

pub fn assess_assembly(instances: &[Instance]) -> AssemblyAssessment {
    let mut result = AssemblyAssessment {
        parts: instances.len(),
        mass_kg: 0.0,
        wheels: 0,
        motors: 0,
        wings: 0,
        gears: 0,
        has_power: false,
        suggested_vehicle: VehicleKind::Car,
        connected_components: 0,
        drive_ready: false,
        reasons: Vec::new(),
    };
    let mut boat = false;
    let mut train = false;
    for instance in instances {
        let Some(part) = part(instance.part_id) else {
            continue;
        };
        result.mass_kg += part.mass();
        result.wheels += usize::from(part.shape == Shape::Wheel);
        result.wings += usize::from(part.shape == Shape::Wing);
        result.gears += usize::from(part.shape == Shape::Gear);
        result.motors += usize::from(matches!(part.id, 140 | 141 | 144));
        result.has_power |= matches!(part.id, 142 | 148 | 128 | 160);
        boat |= part.id == 127;
        train |= matches!(part.id, 104 | 135 | 136);
    }
    result.suggested_vehicle = if boat {
        VehicleKind::Boat
    } else if train {
        VehicleKind::Train
    } else if result.wings >= 2 {
        VehicleKind::Plane
    } else {
        VehicleKind::Car
    };
    result.connected_components = components(instances);
    if !result.has_power {
        result
            .reasons
            .push("Add a battery, solar panel, or sail".into());
    }
    if !boat && result.motors == 0 {
        result.reasons.push("Add a drive motor".into());
    }
    match result.suggested_vehicle {
        VehicleKind::Car if result.wheels < 4 => result.reasons.push("Add four road wheels".into()),
        VehicleKind::Train if result.wheels < 2 => result.reasons.push("Add train wheels".into()),
        VehicleKind::Plane if result.wings < 2 => result.reasons.push("Add a pair of wings".into()),
        _ => {}
    }
    if result.connected_components > 1 {
        result.reasons.push(format!(
            "Connect {} separate part groups",
            result.connected_components
        ));
    }
    result.drive_ready = !instances.is_empty() && result.reasons.is_empty();
    result
}

fn components(instances: &[Instance]) -> usize {
    let mut roots: Vec<usize> = (0..instances.len()).collect();
    fn root(roots: &mut [usize], mut i: usize) -> usize {
        while roots[i] != i {
            roots[i] = roots[roots[i]];
            i = roots[i];
        }
        i
    }
    for a in 0..instances.len() {
        for b in 0..a {
            let pa = instances[a].position;
            let pb = instances[b].position;
            let sa = instances[a].size().map(i32::from);
            let sb = instances[b].size().map(i32::from);
            let connected = (0..3).any(|axis| {
                (pa[axis] + sa[axis] == pb[axis] || pb[axis] + sb[axis] == pa[axis])
                    && (0..3).filter(|other| *other != axis).all(|other| {
                        pa[other] < pb[other] + sb[other] && pb[other] < pa[other] + sa[other]
                    })
            });
            if connected {
                let ra = root(&mut roots, a);
                let rb = root(&mut roots, b);
                roots[ra] = rb;
            }
        }
    }
    (0..instances.len())
        .filter(|i| root(&mut roots, *i) == *i)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use briccraft_world::{Blueprint, World};
    #[test]
    fn functional_blueprints_have_connected_powered_drives() {
        for blueprint in [
            Blueprint::Rover,
            Blueprint::Airplane,
            Blueprint::Locomotive,
            Blueprint::Sailboat,
        ] {
            let mut world = World::empty(1);
            world.spawn_blueprint(blueprint, [20, 0, 20], 0).unwrap();
            let assessment = assess_assembly(&world.instances().cloned().collect::<Vec<_>>());
            assert!(
                assessment.drive_ready,
                "{blueprint:?}: {:?}",
                assessment.reasons
            );
        }
    }
    #[test]
    fn disconnected_power_is_rejected() {
        let mut world = World::empty(1);
        let ids = world
            .spawn_blueprint(Blueprint::Rover, [20, 0, 20], 0)
            .unwrap();
        let mut instances: Vec<_> = ids
            .into_iter()
            .map(|id| world.instance(id).unwrap().clone())
            .collect();
        instances
            .iter_mut()
            .find(|instance| instance.part_id == 160)
            .unwrap()
            .position = [80, 0, 80];
        let assessment = assess_assembly(&instances);
        assert_eq!(assessment.connected_components, 2);
        assert!(!assessment.drive_ready);
    }
}
