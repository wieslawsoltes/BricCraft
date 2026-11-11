use crate::{Blueprint, BuildOp, Placement, World};

impl World {
    /// Seeded brick archipelago, assembled from the same editable catalog parts as player builds.
    pub fn generate(seed: u64) -> Self {
        let mut world = Self::empty(seed);
        terrain(&mut world);
        settlement(&mut world);
        railway(&mut world);
        foliage(&mut world);
        world
    }
}

fn hash(seed: u64, x: i32, z: i32) -> u64 {
    let mut h = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (z as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h = (h ^ (h >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^ (h >> 31)
}
fn random(seed: u64, x: i32, z: i32) -> f32 {
    (hash(seed, x, z) >> 40) as f32 / 16_777_215.0
}

fn height(seed: u64, x: i32, z: i32) -> i32 {
    let dx = (x as f32 - 47.0) / 39.0;
    let dz = (z as f32 - 47.0) / 35.0;
    let radius = (dx * dx + dz * dz).sqrt();
    let edge = (x as f32 * 0.18).sin() * 0.06 + (z as f32 * 0.13).cos() * 0.05;
    if radius > 1.12 + edge {
        return 0;
    }
    let mut h = ((1.12 + edge - radius) * 21.0 + 3.0).clamp(3.0, 18.0) as i32 / 3 * 3;
    // Broad terraced building meadow and village, leaving the shoreline irregular.
    if (26..72).contains(&x) && (24..70).contains(&z) {
        h = 18;
    }
    // A wooded lookout rises behind the village, away from the beginner building area.
    let hill = ((x - 70).pow(2) + (z - 34).pow(2)) as f32;
    if hill < 150.0 {
        h = h.max(18 + (3.0 * (1.0 - hill / 150.0)).ceil() as i32 * 3);
    }
    // A low ledge beneath the elevated coastal railway.
    if (16..80).contains(&x)
        && (16..80).contains(&z)
        && ((16..26).contains(&x)
            || (70..80).contains(&x)
            || (16..26).contains(&z)
            || (70..80).contains(&z))
    {
        h = h.min(15);
    }
    if random(seed, x / 6, z / 6) > 0.9 && h < 15 {
        h = (h - 3).max(3);
    }
    h
}

fn terrain(world: &mut World) {
    let mut placements = Vec::new();
    for z in (0..96).step_by(2) {
        for x in (0..96).step_by(2) {
            let h = height(world.seed, x, z);
            for y in (0..h).step_by(3) {
                let top = y + 3 == h;
                let color = if top {
                    if h <= 9 {
                        if random(world.seed.wrapping_add(1), x, z) > 0.7 {
                            26
                        } else {
                            10
                        }
                    } else if random(world.seed.wrapping_add(2), x, z) > 0.87 {
                        21
                    } else {
                        12
                    }
                } else if h - y <= 6 {
                    9
                } else {
                    31
                };
                placements.push(BuildOp::Place(Placement::new(2, color, [x, y, z], 0)));
            }
        }
    }
    world
        .apply(&placements)
        .expect("non-overlapping generated terrain");
}

fn try_part(world: &mut World, part: u16, color: u8, position: [i32; 3], rotation: u8) {
    let _ = world.place(Placement::new(part, color, position, rotation));
}

fn settlement(world: &mut World) {
    // A honey-colored pedestrian promenade leads through the village to the building meadow.
    for z in (26..68).step_by(2) {
        for x in [40, 42, 44, 46] {
            try_part(world, 42, 10, [x, 18, z], 0);
        }
    }
    for z in [34, 36] {
        for x in (26..68).step_by(2) {
            try_part(world, 42, 10, [x, 18, z], 0);
        }
    }
    for (origin, color, rotation) in [
        ([28, 18, 25], 14, 0),
        ([50, 18, 25], 16, 0),
        ([54, 18, 44], 13, 3),
    ] {
        let ops: Vec<_> = Blueprint::House
            .placements(origin, rotation)
            .into_iter()
            .map(|mut placement| {
                if placement.color == 3 {
                    placement.color = color;
                }
                BuildOp::Place(placement)
            })
            .collect();
        world
            .apply(&ops)
            .expect("validated cottage on clear building pad");
    }
    // Lighthouse on a small stone foundation at the southwest headland.
    for z in (64..70).step_by(2) {
        for x in (28..34).step_by(2) {
            let current = (world.surface_height(x as f32, z as f32) / 0.4).round() as i32;
            for y in (current..18).step_by(3) {
                try_part(world, 2, 6, [x, y, z], 0);
            }
        }
    }
    let _ = world.spawn_blueprint(Blueprint::Tower, [28, 18, 64], 0);
    for (x, z) in [(39, 28), (48, 28), (39, 44), (48, 44), (39, 60), (48, 60)] {
        try_part(world, 152, 4, [x, 19, z], 0);
        try_part(world, 153, 30, [x, 34, z], 0);
    }
    for z in [40, 52, 62] {
        try_part(world, 0, 9, [36, 18, z], 0);
        try_part(world, 0, 9, [36, 18, z + 4], 0);
        try_part(world, 150, 28, [36, 21, z], 0);
    }
    // Open plaza: paving perimeter only, preserving stud connections inside.
    for x in (48..62).step_by(2) {
        for z in [48, 62] {
            try_part(world, 42, 6, [x, 18, z], 0);
        }
    }
    for z in (50..62).step_by(2) {
        for x in [48, 60] {
            try_part(world, 42, 6, [x, 18, z], 0);
        }
    }
    // A red rover gives an immediate sense of scale next to the main path.
    let _ = world.spawn_blueprint(Blueprint::Rover, [50, 19, 53], 0);
    // Harbor boardwalk and mooring poles on the west shore.
    for z in (38..58).step_by(4) {
        try_part(world, 26, 28, [10, 9, z], 0);
    }
    for z in [38, 46, 54] {
        try_part(world, 69, 9, [9, 0, z], 0);
    }
    let _ = world.spawn_blueprint(Blueprint::Sailboat, [2, 5, 41], 0);
}

fn railway(world: &mut World) {
    // Raised plate roadbed allows a track to traverse changing terrain continuously.
    for z in (24..72).step_by(4) {
        for x in [18, 72] {
            try_part(world, 26, 7, [x, 18, z], 0);
            for rail_x in [x, x + 3] {
                try_part(world, 46, 6, [rail_x, 19, z], 0);
            }
        }
    }
    for x in (24..72).step_by(4) {
        for z in [18, 72] {
            try_part(world, 26, 7, [x, 18, z], 0);
            for rail_z in [z, z + 3] {
                try_part(world, 46, 6, [x, 19, rail_z], 1);
            }
        }
    }
    for (x, z) in [(18, 18), (18, 72), (72, 18), (72, 72)] {
        for dx in [0, 4] {
            for dz in [0, 4] {
                try_part(world, 26, 7, [x + dx, 18, z + dz], 0);
            }
        }
        for d in 0..6 {
            try_part(world, 40, 6, [x + d, 19, z + d], 0);
            try_part(world, 40, 6, [x + d, 19, z + d + 2], 0);
        }
    }
    for (x, z) in [
        (18, 30),
        (18, 46),
        (18, 62),
        (72, 30),
        (72, 46),
        (72, 62),
        (30, 18),
        (46, 18),
        (62, 18),
        (30, 72),
        (46, 72),
        (62, 72),
    ] {
        let bottom = (world.surface_height(x as f32, z as f32) / 0.4).round() as i32;
        // Read the pre-roadbed column below it, rather than attempting overlapping support columns.
        for y in (0..18).step_by(3) {
            if bottom > 0 {
                try_part(world, 2, 6, [x, y, z], 0);
            }
        }
    }
}

fn foliage(world: &mut World) {
    for z in (12..84).step_by(5) {
        for x in (12..84).step_by(5) {
            if random(world.seed.wrapping_add(10), x, z) < 0.40 {
                continue;
            }
            if (25..66).contains(&x) && (23..70).contains(&z) {
                continue;
            }
            if (16..27).contains(&x)
                || (69..81).contains(&x)
                || (16..27).contains(&z)
                || (69..81).contains(&z)
            {
                continue;
            }
            let y = (world.surface_height(x as f32, z as f32) / 0.4).round() as i32;
            if !(12..32).contains(&y) {
                continue;
            }
            let pine = random(world.seed.wrapping_add(11), x, z) > 0.55;
            let mut ops = vec![BuildOp::Place(Placement::new(115, 28, [x, y, z], 0))];
            if pine {
                ops.push(BuildOp::Place(Placement::new(
                    112,
                    5,
                    [x - 1, y + 9, z - 1],
                    0,
                )));
                ops.push(BuildOp::Place(Placement::new(67, 5, [x, y + 19, z], 0)));
            } else {
                ops.push(BuildOp::Place(Placement::new(
                    111,
                    5,
                    [x - 2, y + 9, z - 2],
                    0,
                )));
                ops.push(BuildOp::Place(Placement::new(
                    110,
                    12,
                    [x - 1, y + 12, z - 1],
                    0,
                )));
                ops.push(BuildOp::Place(Placement::new(61, 21, [x, y + 15, z], 0)));
            }
            let _ = world.apply(&ops);
        }
    }
    for z in (28..70).step_by(3) {
        for x in (27..69).step_by(3) {
            if random(world.seed.wrapping_add(20), x, z) > 0.23 {
                continue;
            }
            let y = (world.surface_height(x as f32, z as f32) / 0.4).round() as i32;
            if y != 18
                || ((38..50).contains(&x))
                || ((46..64).contains(&x) && (46..65).contains(&z))
            {
                continue;
            }
            try_part(world, 114, 5, [x, y, z], 0);
            try_part(
                world,
                113,
                [0, 1, 19, 30][(hash(world.seed, x, z) % 4) as usize],
                [x, y + 2, z],
                0,
            );
        }
    }
}
