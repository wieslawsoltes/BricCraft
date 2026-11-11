use crate::{
    BiomePreset, CHUNK_CELLS, ChunkCoord, GlobalPart, GlobalPlacement, PartKey, StreamError,
    TerrainSample,
};
use briccraft_world::Placement;

#[derive(Debug)]
pub(crate) struct Chunk {
    pub parts: Vec<GlobalPart>,
    pub occupied_cells: usize,
}

pub(crate) fn hash(seed: u64, x: i64, z: i64) -> u64 {
    let mut n = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (z as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    n = (n ^ (n >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    n = (n ^ (n >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    n ^ (n >> 31)
}

fn noise(seed: u64, x: i64, z: i64, scale: i64) -> f32 {
    let ix = x.div_euclid(scale);
    let iz = z.div_euclid(scale);
    let blend = |t: f32| t * t * (3.0 - 2.0 * t);
    let tx = blend(x.rem_euclid(scale) as f32 / scale as f32);
    let tz = blend(z.rem_euclid(scale) as f32 / scale as f32);
    let value = |dx: i64, dz: i64| {
        (hash(seed, ix.wrapping_add(dx), iz.wrapping_add(dz)) >> 40) as f32 / 16_777_215.0
    };
    let a = value(0, 0) * (1.0 - tx) + value(1, 0) * tx;
    let b = value(0, 1) * (1.0 - tx) + value(1, 1) * tx;
    a * (1.0 - tz) + b * tz
}

pub(crate) fn terrain(seed: u64, preset: BiomePreset, x: i64, z: i64) -> TerrainSample {
    // Quantization uses Euclidean division so negative chunks share exactly the same lattice.
    let x = x.div_euclid(2) * 2;
    let z = z.div_euclid(2) * 2;
    let broad = noise(seed, x, z, 128);
    let detail = noise(seed.wrapping_add(17), x, z, 32);
    let fine = noise(seed.wrapping_add(73), x, z, 12);
    let value = match preset {
        BiomePreset::Meadow => 12.0 + broad * 12.0 + detail * 6.0 + fine * 2.0,
        BiomePreset::Alpine => {
            12.0 + (1.0 - (broad * 2.0 - 1.0).abs()) * 30.0 + detail * 12.0 + fine * 5.0
        }
        BiomePreset::Desert => 6.0 + broad * 13.0 + detail * detail * 18.0 + fine * 2.0,
        BiomePreset::Coast => 1.0 + (broad - 0.30).max(0.0) * 36.0 + (detail - 0.5) * 10.0,
        BiomePreset::Flat => 18.0,
    };
    let mut height = ((value / 3.0).round() as i32 * 3).clamp(3, 54);
    if preset == BiomePreset::Meadow && (16..80).contains(&x) && (16..80).contains(&z) {
        height = 18;
    }
    let chunk = ChunkCoord::containing(x, z);
    let lx = x.rem_euclid(16);
    let lz = z.rem_euclid(16);
    if let Some(h) = landmark_height(seed, preset, chunk) {
        if (2..14).contains(&lx) && (2..14).contains(&lz) {
            height = h;
        }
    }
    let variant = hash(seed.wrapping_add(201), x.div_euclid(6), z.div_euclid(6));
    let color = match preset {
        BiomePreset::Flat => 12,
        BiomePreset::Meadow => {
            if variant % 13 == 0 {
                29
            } else {
                12
            }
        }
        BiomePreset::Alpine => {
            if height >= 42 {
                3
            } else if height >= 30 {
                6
            } else {
                25
            }
        }
        BiomePreset::Desert => [8, 10, 26][(variant % 3) as usize],
        BiomePreset::Coast => {
            if height <= 9 {
                10
            } else if variant % 7 == 0 {
                21
            } else {
                12
            }
        }
    };
    TerrainSample {
        height_plates: height,
        color,
        biome: preset,
    }
}

fn landmark_height(seed: u64, preset: BiomePreset, chunk: ChunkCoord) -> Option<i32> {
    if preset == BiomePreset::Flat || hash(seed.wrapping_add(4401), chunk.x, chunk.z) % 27 != 0 {
        return None;
    }
    // No landmarks obstruct the initial meadow's broad building/aircraft area.
    if preset == BiomePreset::Meadow && (1..5).contains(&chunk.x) && (1..5).contains(&chunk.z) {
        return None;
    }
    let origin = chunk.origin().ok()?;
    let x = origin.x.checked_add(8)?;
    let z = origin.z.checked_add(8)?;
    let n = noise(seed, x, z, 128);
    let h = match preset {
        BiomePreset::Meadow => 18 + (n * 3.0).round() as i32 * 3,
        BiomePreset::Alpine => 24 + (n * 4.0).round() as i32 * 3,
        BiomePreset::Desert => 15 + (n * 3.0).round() as i32 * 3,
        BiomePreset::Coast => {
            if n > 0.53 {
                15
            } else {
                return None;
            }
        }
        BiomePreset::Flat => return None,
    };
    Some(h)
}

pub(crate) fn generate(
    seed: u64,
    preset: BiomePreset,
    coord: ChunkCoord,
) -> Result<Chunk, StreamError> {
    let origin = coord.origin()?;
    // A full chunk needs a representable exclusive end, including at the i64 limit.
    origin
        .x
        .checked_add(16)
        .ok_or(StreamError::CoordinateOverflow)?;
    origin
        .z
        .checked_add(16)
        .ok_or(StreamError::CoordinateOverflow)?;
    let mut builder = Builder {
        coord,
        parts: Vec::new(),
        occupied: vec![false; CHUNK_CELLS],
    };
    let samples: Vec<_> = (0..64)
        .map(|i| {
            terrain(
                seed,
                preset,
                origin.x + (i % 8 * 2) as i64,
                origin.z + (i / 8 * 2) as i64,
            )
        })
        .collect();
    let maximum = samples.iter().map(|s| s.height_plates).max().unwrap_or(3);
    for y in (0..maximum).step_by(3) {
        let mut colors = [None; 64];
        for (i, s) in samples.iter().enumerate() {
            if s.height_plates >= y + 3 {
                colors[i] = Some(if s.height_plates == y + 3 {
                    s.color
                } else {
                    match preset {
                        BiomePreset::Desert | BiomePreset::Coast => 10,
                        BiomePreset::Alpine => 7,
                        _ => 11,
                    }
                });
            }
        }
        // Greedily combine only identical horizontal courses. Every resulting terrain part stays
        // inside its chunk, and boundaries remain individually editable brick connections.
        for z in 0..8 {
            for x in 0..8 {
                let Some(color) = colors[z * 8 + x] else {
                    continue;
                };
                for (w, d, part_id, rotation) in [
                    (4, 2, 13, 1),
                    (2, 4, 13, 0),
                    (2, 2, 11, 0),
                    (2, 1, 3, 1),
                    (1, 2, 3, 0),
                    (1, 1, 2, 0),
                ] {
                    if x + w > 8
                        || z + d > 8
                        || !(z..z + d).all(|v| (x..x + w).all(|u| colors[v * 8 + u] == Some(color)))
                    {
                        continue;
                    }
                    builder.push(Placement::new(
                        part_id,
                        color,
                        [(x * 2) as i32, y, (z * 2) as i32],
                        rotation,
                    ));
                    for v in z..z + d {
                        for u in x..x + w {
                            colors[v * 8 + u] = None;
                        }
                    }
                    break;
                }
            }
        }
    }
    if let Some(h) = landmark_height(seed, preset, coord) {
        add_landmark(&mut builder, preset, h, hash(seed, coord.x, coord.z));
    } else if preset != BiomePreset::Flat {
        decorate(&mut builder, seed, preset, &samples);
    }
    let occupied_cells = builder.occupied.iter().filter(|&&v| v).count();
    Ok(Chunk {
        parts: builder.parts,
        occupied_cells,
    })
}

struct Builder {
    coord: ChunkCoord,
    parts: Vec<GlobalPart>,
    occupied: Vec<bool>,
}

impl Builder {
    fn cells(p: Placement) -> Option<Vec<usize>> {
        let s = p.size().map(i32::from);
        if p.position
            .iter()
            .zip(s)
            .zip([16, 72, 16])
            .any(|((&v, s), limit)| v < 0 || v + s > limit)
        {
            return None;
        }
        let mut cells = Vec::with_capacity(s.into_iter().product::<i32>() as usize);
        for y in p.position[1]..p.position[1] + s[1] {
            for z in p.position[2]..p.position[2] + s[2] {
                for x in p.position[0]..p.position[0] + s[0] {
                    cells.push((y * 16 * 16 + z * 16 + x) as usize);
                }
            }
        }
        Some(cells)
    }
    fn push(&mut self, p: Placement) -> bool {
        self.assembly(&[p])
    }
    fn assembly(&mut self, items: &[Placement]) -> bool {
        let mut pending = std::collections::HashSet::new();
        for &p in items {
            let Some(cells) = Self::cells(p) else {
                return false;
            };
            for cell in cells {
                if self.occupied[cell] || !pending.insert(cell) {
                    return false;
                }
            }
        }
        let origin = self.coord.origin().expect("validated chunk origin");
        for &p in items {
            let index = self.parts.len() as u16;
            self.parts.push(GlobalPart {
                key: PartKey::Generated {
                    chunk: self.coord,
                    index,
                },
                placement: GlobalPlacement::from_local(origin, p)
                    .expect("chunk-contained placement"),
            });
        }
        for cell in pending {
            self.occupied[cell] = true;
        }
        true
    }
}

fn decorate(builder: &mut Builder, seed: u64, preset: BiomePreset, samples: &[TerrainSample]) {
    let chunk = builder.coord;
    let variation = hash(seed.wrapping_add(511), chunk.x, chunk.z);
    let initial_meadow =
        preset == BiomePreset::Meadow && (1..5).contains(&chunk.x) && (1..5).contains(&chunk.z);
    for (i, (x, z)) in [(3, 3), (10, 10), (3, 10), (10, 3)].into_iter().enumerate() {
        let rng = hash(variation.wrapping_add(i as u64), x, z);
        let h = samples[(z as usize / 2) * 8 + x as usize / 2].height_plates;
        if h < 9 || initial_meadow {
            continue;
        }
        let mut p = Vec::new();
        match preset {
            BiomePreset::Meadow if rng % 5 < 2 => {
                p.push(Placement::new(115, 28, [x as i32 + 1, h, z as i32 + 1], 0));
                p.push(Placement::new(
                    111,
                    if rng & 8 == 0 { 5 } else { 12 },
                    [x as i32 - 1, h + 9, z as i32 - 1],
                    0,
                ));
                p.push(Placement::new(110, 21, [x as i32, h + 12, z as i32], 0));
            }
            BiomePreset::Alpine if rng % 3 == 0 && h < 39 => {
                p.push(Placement::new(60, 9, [x as i32 + 1, h, z as i32 + 1], 0));
                p.push(Placement::new(112, 25, [x as i32, h + 3, z as i32], 0));
                p.push(Placement::new(
                    67,
                    5,
                    [x as i32 + 1, h + 13, z as i32 + 1],
                    0,
                ));
            }
            BiomePreset::Desert if rng % 5 == 0 => {
                for dy in [0, 3, 6, 9] {
                    p.push(Placement::new(61, 5, [x as i32, h + dy, z as i32], 0));
                }
                // Branch sits beside, not inside, the central column's conservative solid envelope.
                p.push(Placement::new(60, 5, [x as i32 - 1, h + 6, z as i32], 0));
                p.push(Placement::new(60, 5, [x as i32 - 1, h + 9, z as i32], 0));
                p.push(Placement::new(113, 19, [x as i32, h + 12, z as i32], 0));
            }
            BiomePreset::Coast if rng % 4 == 0 && h <= 18 => {
                p.push(Placement::new(115, 9, [x as i32 + 1, h, z as i32 + 1], 0));
                p.push(Placement::new(
                    116,
                    5,
                    [x as i32 - 1, h + 9, z as i32 - 1],
                    0,
                ));
            }
            _ if rng % 4 == 0 => p.push(Placement::new(
                117,
                if preset == BiomePreset::Desert { 16 } else { 6 },
                [x as i32, h, z as i32],
                (rng % 4) as u8,
            )),
            _ => {}
        }
        builder.assembly(&p);
    }
    for i in 0..12 {
        let rng = hash(variation.wrapping_add(801 + i), chunk.x, chunk.z);
        let x = 1 + (rng % 14) as i32;
        let z = 1 + ((rng >> 8) % 14) as i32;
        // Keep a large empty origin meadow, including a plane's full launch footprint.
        if initial_meadow {
            continue;
        }
        let h = samples[(z as usize / 2) * 8 + x as usize / 2].height_plates;
        if h < 9 {
            continue;
        }
        match preset {
            BiomePreset::Meadow => {
                let c = [1, 19, 27, 3][((rng >> 16) % 4) as usize];
                builder.assembly(&[
                    Placement::new(114, 5, [x, h, z], 0),
                    Placement::new(113, c, [x, h + 2, z], 0),
                ]);
            }
            BiomePreset::Alpine if h < 36 => {
                builder.push(Placement::new(118, 25, [x, h, z], 0));
            }
            BiomePreset::Alpine if rng % 4 == 0 => {
                builder.push(Placement::new(119, 22, [x, h, z], 0));
            }
            BiomePreset::Desert if rng % 4 == 0 => {
                builder.push(Placement::new(57, 31, [x, h, z], (rng % 4) as u8));
            }
            BiomePreset::Coast if h <= 12 => {
                builder.push(Placement::new(
                    63,
                    [3, 19, 30][(rng % 3) as usize],
                    [x, h, z],
                    0,
                ));
            }
            _ => {}
        }
    }
}

fn add_landmark(builder: &mut Builder, preset: BiomePreset, h: i32, variant: u64) {
    let mut p = vec![Placement::new(
        31,
        if preset == BiomePreset::Desert { 26 } else { 6 },
        [4, h, 4],
        0,
    )];
    let body = if preset == BiomePreset::Desert {
        10
    } else if preset == BiomePreset::Alpine {
        7
    } else {
        [0, 14, 1][(variant % 3) as usize]
    };
    let roof = if preset == BiomePreset::Coast { 21 } else { 0 };
    if preset == BiomePreset::Desert {
        // An oasis well: stone curb, visible water, two columns, and a canopy.
        for x in [5, 10] {
            p.push(Placement::new(6, 10, [x, h + 1, 6], 0));
        }
        for z in [5, 10] {
            p.push(Placement::new(6, 10, [6, h + 1, z], 1));
        }
        p.push(Placement::new(45, 13, [6, h + 1, 6], 0));
        for x in [4, 11] {
            p.push(Placement::new(17, 28, [x, h + 1, 7], 0));
        }
    } else {
        // An open pavilion preserves sightlines and offers an editable, connected construction.
        for x in [4, 11] {
            for z in [4, 11] {
                p.push(Placement::new(17, body, [x, h + 1, z], 0));
            }
        }
        for z in [4, 11] {
            p.push(Placement::new(7, body, [5, h + 1, z], 1));
        }
        p.push(Placement::new(
            45,
            if variant & 1 == 0 { 2 } else { 1 },
            [6, h + 1, 6],
            0,
        ));
    }
    p.push(Placement::new(31, roof, [4, h + 16, 4], 0));
    for x in [4, 6, 8, 10] {
        p.push(Placement::new(52, roof, [x, h + 17, 4], 0));
        p.push(Placement::new(52, roof, [x, h + 17, 8], 2));
    }
    builder.assembly(&p);
}
