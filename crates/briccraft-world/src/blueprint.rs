use crate::{BuildError, BuildOp, Placement, World};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blueprint {
    Rover,
    Airplane,
    Locomotive,
    Sailboat,
    House,
    Tower,
    Windmill,
    Bridge,
}

impl Blueprint {
    pub const ALL: [Self; 8] = [
        Self::Rover,
        Self::Airplane,
        Self::Locomotive,
        Self::Sailboat,
        Self::House,
        Self::Tower,
        Self::Windmill,
        Self::Bridge,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::Rover => "Trail rover",
            Self::Airplane => "Island hopper",
            Self::Locomotive => "Coastal express",
            Self::Sailboat => "Lagoon sailboat",
            Self::House => "Harbor cottage",
            Self::Tower => "Lighthouse",
            Self::Windmill => "Windmill",
            Self::Bridge => "Stone bridge",
        }
    }
    pub fn placements(self, origin: [i32; 3], rotation: u8) -> Vec<Placement> {
        let mut items = Vec::new();
        match self {
            Self::Rover => rover(&mut items),
            Self::Airplane => airplane(&mut items),
            Self::Locomotive => locomotive(&mut items),
            Self::Sailboat => sailboat(&mut items),
            Self::House => house(&mut items),
            Self::Tower => tower(&mut items),
            Self::Windmill => windmill(&mut items),
            Self::Bridge => bridge(&mut items),
        }
        let [width, _, depth] = bounds(&items);
        // All transport blueprints use +Z as their forward axis. Chassis plates have no visual
        // front; keep their canonical orientation as the assembly's heading reference.
        if matches!(self, Self::Rover | Self::Airplane | Self::Locomotive) {
            for item in &mut items {
                let size = item.size().map(i32::from);
                item.position[0] = width - item.position[0] - size[0];
                item.position[2] = depth - item.position[2] - size[2];
                item.rotation = (item.rotation + 2) & 3;
                if matches!(item.part_id, 120 | 135) {
                    item.rotation = 0;
                }
            }
        }
        for item in &mut items {
            let [x, y, z] = item.position;
            let size = item.size().map(i32::from);
            let position = match rotation & 3 {
                0 => [x, y, z],
                1 => [z, y, width - x - size[0]],
                2 => [width - x - size[0], y, depth - z - size[2]],
                _ => [depth - z - size[2], y, x],
            };
            item.position = std::array::from_fn(|axis| origin[axis] + position[axis]);
            item.rotation = (item.rotation + rotation) & 3;
        }
        items
    }
    pub fn size(self, rotation: u8) -> [i32; 3] {
        bounds(&self.placements([0; 3], rotation))
    }
}

fn bounds(items: &[Placement]) -> [i32; 3] {
    let mut size = [0; 3];
    for item in items {
        let s = item.size();
        for axis in 0..3 {
            size[axis] = size[axis].max(item.position[axis] + s[axis] as i32);
        }
    }
    size
}
fn add(items: &mut Vec<Placement>, part: u16, color: u8, p: [i32; 3], rotation: u8) {
    items.push(Placement::new(part, color, p, rotation));
}

fn rover(p: &mut Vec<Placement>) {
    add(p, 120, 7, [2, 6, 0], 0);
    for x in [0, 6] {
        for z in [1, 6] {
            add(p, 101, 4, [x, 0, z], 0);
        }
    }
    add(p, 11, 0, [2, 8, 0], 0);
    add(p, 3, 0, [2, 8, 8], 1);
    for x in [2, 5] {
        add(p, 0, 0, [x, 8, 4], 0);
        add(p, 70, 20, [x, 8, 5], 0);
        add(p, 0, 0, [x, 8, 7], 0);
    }
    add(p, 70, 20, [3, 8, 4], 1);
    add(p, 70, 20, [3, 8, 7], 1);
    add(p, 26, 0, [2, 14, 4], 0);
    add(p, 140, 7, [3, 1, 3], 0);
    add(p, 160, 7, [3, 8, 5], 0);
    for x in [2, 5] {
        add(p, 63, 1, [x, 11, 0], 0);
        add(p, 63, 15, [x, 11, 9], 0);
    }
    add(p, 48, 7, [0, 5, 0], 1);
}

fn airplane(p: &mut Vec<Placement>) {
    for z in [2, 12] {
        add(p, 120, 3, [8, 4, z], 0);
    }
    add(p, 122, 1, [0, 5, 8], 1);
    add(p, 122, 1, [12, 5, 8], 1);
    for z in [4, 8, 12, 16] {
        add(p, 3, 3, [9, 6, z], 0);
    }
    add(p, 126, 20, [8, 9, 5], 0);
    add(p, 124, 0, [9, 10, 18], 0);
    add(p, 28, 0, [6, 9, 18], 1);
    add(p, 125, 4, [7, 0, 0], 1);
    for x in [7, 12] {
        add(p, 100, 4, [x, 0, 8], 0);
    }
    add(p, 140, 7, [9, 6, 1], 0);
    add(p, 160, 7, [9, 9, 12], 0);
}

fn locomotive(p: &mut Vec<Placement>) {
    add(p, 135, 7, [1, 5, 0], 0);
    for x in [0, 7] {
        for z in [2, 6, 11] {
            add(p, 104, 4, [x, 0, z], 0);
        }
    }
    for z in [3, 7, 12] {
        add(p, 88, 7, [1, 2, z], 1);
        add(p, 20, 7, [3, 3, z], 0);
        add(p, 20, 7, [3, 4, z], 0);
    }
    for z in [1, 5] {
        for y in [7, 10] {
            add(p, 62, 15, [2, y, z], 0);
        }
    }
    add(p, 136, 4, [3, 13, 1], 0);
    for x in [1, 6] {
        add(p, 71, 3, [x, 7, 10], 0);
    }
    for x in [2, 4] {
        add(p, 70, 20, [x, 7, 10], 1);
    }
    add(p, 30, 0, [1, 16, 9], 0);
    add(p, 142, 7, [2, 7, 11], 0);
    add(p, 140, 7, [3, 10, 11], 0);
    add(p, 133, 0, [2, 2, 0], 0);
    add(p, 134, 7, [3, 5, 16], 0);
}

fn sailboat(p: &mut Vec<Placement>) {
    add(p, 127, 3, [0, 0, 0], 0);
    add(p, 28, 10, [1, 4, 2], 0);
    add(p, 17, 3, [3, 5, 5], 0);
    add(p, 17, 3, [3, 20, 5], 0);
    add(p, 128, 1, [3, 8, 6], 0);
    add(p, 129, 0, [2, 5, 8], 0);
    add(p, 138, 7, [2, 0, 12], 0);
    add(p, 134, 9, [2, 6, 12], 0);
}

fn house(p: &mut Vec<Placement>) {
    add(p, 31, 10, [0, 0, 0], 0);
    // A real hollow interior; the door frame and windows occupy the wall only.
    for y in [1, 4, 7, 10, 13, 16] {
        add(p, 8, 3, [0, y, 0], 0);
        add(p, 8, 3, [7, y, 0], 0);
        add(p, 7, 3, [1, y, 7], 1);
    }
    add(p, 72, 0, [2, 1, 0], 1);
    for x in [1, 6] {
        for y in [1, 4, 7, 10, 13, 16] {
            add(p, 0, 3, [x, y, 0], 0);
        }
    }
    for x in [0, 2, 4, 6] {
        add(p, 52, 15, [x, 19, 0], 0);
        add(p, 52, 15, [x, 19, 4], 2);
    }
    for x in [0, 2, 4, 6] {
        add(p, 3, 15, [x, 22, 2], 0);
    }
    add(p, 154, 9, [2, 1, 4], 0);
    add(p, 151, 10, [2, 4, 2], 0);
    add(p, 0, 7, [6, 22, 6], 0);
    add(p, 0, 7, [6, 25, 6], 0);
}

fn tower(p: &mut Vec<Placement>) {
    add(p, 29, 6, [0, 0, 0], 0);
    for level in 0..10 {
        let color = if level % 4 < 2 { 3 } else { 0 };
        add(p, 62, color, [1, 1 + level * 3, 1], 0);
    }
    add(p, 29, 4, [0, 31, 0], 0);
    for x in [1, 4] {
        for z in [1, 4] {
            add(p, 17, 3, [x, 32, z], 0);
        }
    }
    add(p, 61, 1, [2, 35, 2], 0);
    add(p, 29, 4, [0, 47, 0], 0);
    add(p, 68, 0, [1, 48, 1], 0);
}

fn windmill(p: &mut Vec<Placement>) {
    add(p, 29, 10, [0, 0, 0], 0);
    for y in [1, 4, 7, 10, 13, 16, 19] {
        add(p, 62, 3, [1, y, 1], 0);
    }
    add(p, 68, 15, [1, 22, 1], 0);
    add(p, 125, 3, [2, 10, 6], 0);
    add(p, 140, 7, [3, 18, 6], 0);
}

fn bridge(p: &mut Vec<Placement>) {
    for z in [0, 6, 12] {
        add(p, 75, 6, [0, 0, z], 1);
    }
    for x in [0, 4] {
        for z in [0, 8] {
            add(p, 28, 6, [x, 12, z], 0);
        }
    }
    for x in [0, 7] {
        for z in [0, 4, 8, 12] {
            add(p, 76, 3, [x, 13, z], 0);
        }
    }
}

impl World {
    pub fn spawn_blueprint(
        &mut self,
        blueprint: Blueprint,
        origin: [i32; 3],
        rotation: u8,
    ) -> Result<Vec<u32>, BuildError> {
        let operations: Vec<_> = blueprint
            .placements(origin, rotation)
            .into_iter()
            .map(BuildOp::Place)
            .collect();
        Ok(self
            .apply(&operations)?
            .added
            .iter()
            .map(|part| part.id)
            .collect())
    }
}
