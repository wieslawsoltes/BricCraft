//! One assessment path for both the workshop UI and actual vehicle activation.
use crate::Session;
use briccraft_mechanics::{assess_assembly, AssemblyAssessment, VehicleKind};
use briccraft_world::Instance;

#[derive(Debug, Clone)]
pub struct WorkshopStatus {
    pub title: String,
    pub detail: String,
    pub ready: bool,
}

pub(super) struct PreparedVehicle {
    pub ids: Vec<u32>,
    pub instances: Vec<Instance>,
    pub assessment: AssemblyAssessment,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub kind: VehicleKind,
}
impl Session {
    pub fn workshop_status(&self) -> WorkshopStatus {
        if let Some(active) = &self.active_vehicle {
            return WorkshopStatus{title:"Vehicle active".into(),detail:format!("{} connected parts are moving together. Park on clear ground before editing the construction.",active.parts.len()),ready:true};
        }
        match self.prepare_vehicle("auto") {
            Ok(ready)=>WorkshopStatus{
                title:match ready.kind {VehicleKind::Plane=>"Ready for takeoff",VehicleKind::Boat=>"Ready to sail",VehicleKind::Train=>"Ready for the railway",VehicleKind::Car=>"Ready to drive"}.into(),
                detail:format!("{} connected parts · {} motor(s) · {} wheels · {:.2} kg · {:.2}:1 transmission. Activate the vehicle to use its controls.",
                    ready.assessment.parts,ready.assessment.motors,ready.assessment.wheels,ready.assessment.mass_kg,ready.assessment.drive_ratio),ready:true,
            },
            Err(detail)=>WorkshopStatus{title:"Workshop check".into(),detail,ready:false},
        }
    }

    pub(super) fn prepare_vehicle(&self, requested: &str) -> Result<PreparedVehicle, String> {
        let pointed = self
            .hit
            .as_ref()
            .map(|h| self.world.connected_assembly(h.id, 256))
            .unwrap_or_default();
        let pointed_parts: Vec<_> = pointed
            .iter()
            .filter_map(|id| self.world.instance(*id).cloned())
            .collect();
        let pointed_ready = pointed.len() < 256 && assess_assembly(&pointed_parts).drive_ready;
        let last: Vec<_> = self
            .last_blueprint
            .iter()
            .copied()
            .filter(|id| self.world.instance(*id).is_some())
            .collect();
        let ids = if pointed_ready {
            pointed
        } else if !last.is_empty() {
            last
        } else {
            pointed
        };
        if ids.is_empty() {
            return Err(
                "Insert a vehicle blueprint, or point at your connected vehicle construction."
                    .into(),
            );
        }
        let instances: Vec<_> = ids
            .iter()
            .filter_map(|id| self.world.instance(*id).cloned())
            .collect();
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        let mut wheels = 0;
        let mut propellers = 0;
        let mut wings = 0;
        let mut hulls = 0;
        for instance in &instances {
            let p = briccraft_parts::part(instance.part_id).expect("valid world part");
            wheels += usize::from(p.shape == briccraft_parts::Shape::Wheel);
            propellers += usize::from(p.shape == briccraft_parts::Shape::Propeller);
            wings += usize::from(p.shape == briccraft_parts::Shape::Wing);
            hulls += usize::from(p.id == 127);
            let position = instance.world_position();
            let size = instance.size();
            for axis in 0..3 {
                min[axis] = min[axis].min(position[axis]);
                max[axis] = max[axis].max(
                    position[axis] + f32::from(size[axis]) * if axis == 1 { 0.4 } else { 1.0 },
                );
            }
        }
        if ids.len() >= 256 || max[0] - min[0] > 32.0 || max[2] - min[2] > 32.0 {
            return Err("This assembly is connected to the landscape. Build the vehicle as a separate construction or insert a vehicle blueprint.".into());
        }
        let assessment = assess_assembly(&instances);
        if !assessment.drive_ready {
            return Err(format!(
                "Construction needs attention: {}.",
                assessment.reasons.join("; ")
            ));
        }
        let kind = match requested {
            "car" => VehicleKind::Car,
            "plane" => VehicleKind::Plane,
            "train" => VehicleKind::Train,
            "boat" => VehicleKind::Boat,
            _ => assessment.suggested_vehicle,
        };
        if self.map_anchor().is_some() && kind == VehicleKind::Boat {
            return Err("Boats currently need the island sea. Choose a car or aircraft for this mapped world.".into());
        }
        if self.is_streamed() && kind == VehicleKind::Train {
            return Err("Powered trains use the coastal railway in Sunlit Archipelago. This open world has no guided railway network yet; the locomotive remains editable here.".into());
        }
        let complete = match kind {
            VehicleKind::Car | VehicleKind::Train => wheels >= 2,
            VehicleKind::Plane => wings >= 1 && propellers >= 1,
            VehicleKind::Boat => hulls >= 1,
        };
        if !complete {
            return Err("This construction is missing running gear. Cars and trains need wheels; planes need wings and a propeller; boats need a hull.".into());
        }
        Ok(PreparedVehicle {
            ids,
            instances,
            assessment,
            min,
            max,
            kind,
        })
    }
}
