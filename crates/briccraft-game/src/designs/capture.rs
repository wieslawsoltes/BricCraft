use crate::Session;
use briccraft_design::{CaptureKind, MAX_DIMENSIONS, MAX_PARTS};

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaptureKey {
    scene: u64,
    revision: u64,
    hit: Option<u32>,
    last: Vec<u32>,
    global_last_count: usize,
}

#[derive(Debug, Clone)]
pub(super) struct Candidate {
    pub ids: Vec<u32>,
    pub kind: CaptureKind,
    pub label: &'static str,
}

#[derive(Debug, Clone)]
pub(crate) struct CaptureCache {
    key: CaptureKey,
    candidate: Result<Candidate, String>,
}

impl Session {
    pub(super) fn prepare_design_capture(&self) -> Result<Candidate, String> {
        let key = CaptureKey {
            scene: self.scene_generation,
            revision: self.world.revision(),
            hit: self.hit.as_ref().map(|hit| hit.id),
            last: self
                .last_blueprint
                .iter()
                .take(MAX_PARTS + 1)
                .copied()
                .collect(),
            global_last_count: self
                .streaming
                .as_ref()
                .map_or(0, |state| state.last_blueprint_keys.len()),
        };
        if let Some(cache) = self.design_capture_cache.borrow().as_ref() {
            if cache.key == key {
                return cache.candidate.clone();
            }
        }
        let candidate = self.find_design_capture();
        *self.design_capture_cache.borrow_mut() = Some(CaptureCache {
            key,
            candidate: candidate.clone(),
        });
        candidate
    }

    fn find_design_capture(&self) -> Result<Candidate, String> {
        let mut failure = "Point at a separate construction, or insert a blueprint and save its latest construction.".to_owned();
        if let Some(hit) = self.hit {
            let ids = self.world.connected_assembly(hit.id, MAX_PARTS + 1);
            match self.bounded_design_candidate(
                ids,
                CaptureKind::ConnectedAssembly,
                "the pointed connected assembly",
            ) {
                Ok(candidate) => return Ok(candidate),
                Err(error) => failure = error,
            }
        }
        if !self.last_blueprint.is_empty() {
            if self.last_blueprint.len() > MAX_PARTS {
                return Err(format!("The latest construction exceeds {MAX_PARTS} parts. Select a smaller connected build."));
            }
            // A remembered construction is an explicit group, including any separate decorative parts.
            let ids: Vec<_> = self
                .last_blueprint
                .iter()
                .take(MAX_PARTS + 1)
                .copied()
                .filter(|id| self.world.instance(*id).is_some())
                .collect();
            if self
                .streaming
                .as_ref()
                .is_some_and(|state| state.last_blueprint_keys.len() > ids.len())
            {
                return Err("The latest construction is only partly loaded. Move closer to its center before capturing it.".into());
            }
            return self.bounded_design_candidate(
                ids,
                CaptureKind::ConstructionGroup,
                "the latest construction",
            );
        }
        Err(failure)
    }

    fn bounded_design_candidate(
        &self,
        ids: Vec<u32>,
        kind: CaptureKind,
        label: &'static str,
    ) -> Result<Candidate, String> {
        if ids.is_empty() {
            return Err("The selected construction no longer exists. Select another build.".into());
        }
        if ids.len() > MAX_PARTS {
            return Err(format!("This selection reaches the landscape or exceeds {MAX_PARTS} parts. Point at a separate build or use the latest construction."));
        }
        let mut min = [i32::MAX; 3];
        let mut max = [i32::MIN; 3];
        for &id in &ids {
            let instance = self
                .world
                .instance(id)
                .ok_or_else(|| "The selection changed. Select it again.".to_owned())?;
            for axis in 0..3 {
                min[axis] = min[axis].min(instance.position[axis]);
                max[axis] =
                    max[axis].max(instance.position[axis] + i32::from(instance.size()[axis]));
            }
        }
        if (0..3).any(|axis| max[axis] - min[axis] > i32::from(MAX_DIMENSIONS[axis])) {
            return Err("The selection exceeds 32 × 72 × 32 cells or is joined to the landscape. Use a smaller construction.".into());
        }
        if let Some(state) = &self.streaming {
            // Do not silently save a partial assembly at the active volume's boundary.
            if min[0] <= 0 || min[2] <= 0 || max[0] >= 96 || max[2] >= 96 {
                return Err("Move closer to the construction so its whole footprint is inside the loaded area.".into());
            }
            for boundary in &state.snapshot.boundary_parts {
                let p = boundary.placement;
                let size = p.size().map_err(|e| e.to_string())?;
                let x = p
                    .position
                    .x
                    .checked_sub(state.snapshot.origin.x)
                    .ok_or_else(|| "Invalid boundary coordinate.".to_owned())?;
                let z = p
                    .position
                    .z
                    .checked_sub(state.snapshot.origin.z)
                    .ok_or_else(|| "Invalid boundary coordinate.".to_owned())?;
                let start = [x, i64::from(p.position.y), z];
                let end: [i64; 3] = std::array::from_fn(|axis| start[axis] + i64::from(size[axis]));
                // Conservative whole-envelope contact also catches omitted overhangs reaching inward.
                if (0..3).all(|axis| {
                    start[axis] <= i64::from(max[axis]) && end[axis] >= i64::from(min[axis])
                }) {
                    return Err("An unloaded boundary part touches this construction. Move closer before capturing it.".into());
                }
            }
        }
        Ok(Candidate { ids, kind, label })
    }
}
