use crate::{Gameplay, MAX_COUNTER, MAX_STUDS};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy)]
pub enum TravelKind {
    Walk,
    Drive,
    Flight,
}
#[derive(Debug, Clone, Copy)]
pub enum Activity {
    Placed(u64),
    Recycled(u64),
    Structure,
    Activated,
    Travel { kind: TravelKind, studs: f64 },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Statistics {
    placed: u64,
    recycled: u64,
    structures: u64,
    activations: u64,
    walk_millistuds: u64,
    drive_millistuds: u64,
    flight_millistuds: u64,
}
impl Statistics {
    pub fn values(&self) -> [u64; 7] {
        [
            self.placed,
            self.recycled,
            self.structures,
            self.activations,
            self.walk_millistuds,
            self.drive_millistuds,
            self.flight_millistuds,
        ]
    }
}

#[derive(Debug, Clone)]
pub struct ObjectiveView {
    pub id: &'static str,
    pub title: &'static str,
    pub detail: &'static str,
    pub progress: u64,
    pub target: u64,
    pub reward: u64,
    pub completed: bool,
}
struct Objective {
    id: &'static str,
    title: &'static str,
    detail: &'static str,
    target: u64,
    reward: u64,
}
const OBJECTIVES:[Objective;8]=[
    Objective{id:"salvage",title:"Gather supplies",detail:"Recycle 8 parts with Remove. Every part returns its full catalog stud value.",target:8,reward:140},
    Objective{id:"build",title:"Make your mark",detail:"Successfully place 24 parts. Blueprints count as their actual constituent parts.",target:24,reward:180},
    Objective{id:"walk",title:"Find your footing",detail:"Walk 120 studs through the world. Orbiting, camera focus and streaming shifts do not count.",target:120,reward:120},
    Objective{id:"home",title:"Establish a base",detail:"Complete a cottage, lighthouse, windmill or bridge blueprint on clear ground.",target:1,reward:220},
    Objective{id:"workshop",title:"Bring it to life",detail:"Activate a connected, functional vehicle with the required power and running gear.",target:1,reward:260},
    Objective{id:"drive",title:"Follow the trail",detail:"Travel 240 studs in an active vehicle on land or water.",target:240,reward:320},
    Objective{id:"flight",title:"Take to the sky",detail:"Fly a powered airplane 160 studs while clear of the terrain.",target:160,reward:420},
    Objective{id:"journey",title:"A world of possibilities",detail:"Travel 1000 studs by walking, driving, sailing or powered flight.",target:1000,reward:500},
];

#[derive(Debug, Default)]
pub struct Reward {
    pub studs: u64,
    pub objectives: Vec<&'static str>,
}
impl Gameplay {
    fn progress_values(&self) -> [u64; 8] {
        let s = &self.statistics;
        [
            s.recycled,
            s.placed,
            s.walk_millistuds / 1000,
            s.structures,
            s.activations,
            s.drive_millistuds / 1000,
            s.flight_millistuds / 1000,
            (s.walk_millistuds + s.drive_millistuds + s.flight_millistuds) / 1000,
        ]
    }
    pub fn objectives(&self) -> Vec<ObjectiveView> {
        let progress = self.progress_values();
        OBJECTIVES
            .iter()
            .enumerate()
            .map(|(i, o)| ObjectiveView {
                id: o.id,
                title: o.title,
                detail: o.detail,
                progress: progress[i].min(o.target),
                target: o.target,
                reward: o.reward,
                completed: self.claimed & (1 << i) != 0,
            })
            .collect()
    }
    pub fn guidance(&self) -> &'static str {
        if !self.is_expedition() {
            return "Creative has unlimited parts. Start Expedition for a stud budget and construction, travel and engineering objectives.";
        }
        OBJECTIVES.iter().enumerate().find(|(i,_)|self.claimed&(1<<i)==0)
            .map_or("All expedition objectives completed. Your worlds, wallet and constructions remain yours to expand.",|(_,o)|o.detail)
    }
    /// Record only successfully committed player actions, never undo/redo replay.
    /// Travel is measured per simulation step; nonfinite/teleport-sized deltas
    /// cannot accidentally award distance after a rebase or camera teleport.
    pub fn record(&mut self, activity: Activity) -> Reward {
        if !self.is_expedition() {
            return Reward::default();
        }
        let (counter, amount) = match activity {
            Activity::Placed(n) => (&mut self.statistics.placed, n),
            Activity::Recycled(n) => (&mut self.statistics.recycled, n),
            Activity::Structure => (&mut self.statistics.structures, 1),
            Activity::Activated => (&mut self.statistics.activations, 1),
            Activity::Travel { kind, studs } => {
                if !studs.is_finite() || studs <= 0.0 || studs > 8.0 {
                    return Reward::default();
                }
                let counter = match kind {
                    TravelKind::Walk => &mut self.statistics.walk_millistuds,
                    TravelKind::Drive => &mut self.statistics.drive_millistuds,
                    TravelKind::Flight => &mut self.statistics.flight_millistuds,
                };
                (counter, (studs * 1000.0).round() as u64)
            }
        };
        *counter = counter.saturating_add(amount).min(MAX_COUNTER);
        let progress = self.progress_values();
        let mut reward = Reward::default();
        for (i, objective) in OBJECTIVES.iter().enumerate() {
            let bit = 1 << i;
            if self.claimed & bit == 0 && progress[i] >= objective.target {
                self.claimed |= bit;
                let next = self.studs.saturating_add(objective.reward).min(MAX_STUDS);
                reward.studs += next - self.studs;
                self.studs = next;
                reward.objectives.push(objective.title);
            }
        }
        self.revision = self.revision.wrapping_add(1);
        reward
    }
}
