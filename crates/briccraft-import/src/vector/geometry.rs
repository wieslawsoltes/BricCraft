use super::{
    protobuf::{zigzag, Reader},
    TileRange,
};
use crate::{error::invalid, limits::limit, ImportResult};

pub struct Path {
    pub coordinates: Vec<[f64; 2]>,
    pub exterior: bool,
}

pub fn decode(
    bytes: &[u8],
    kind: u64,
    extent: u32,
    tile: &TileRange,
    vertices: &mut usize,
    maximum: usize,
) -> ImportResult<Vec<Path>> {
    let mut reader = Reader::new(bytes);
    let mut cursor = [0i64; 2];
    let mut points = Vec::<[i64; 2]>::new();
    let mut paths = Vec::new();
    while !reader.empty() {
        let command = reader.varint()?;
        let id = command & 7;
        let count =
            usize::try_from(command >> 3).map_err(|_| invalid("MVT command count overflows"))?;
        if count == 0 {
            return Err(invalid("MVT commands must have positive counts"));
        }
        match id {
            1 | 2 => {
                if id == 1 {
                    if count != 1 {
                        return Err(invalid("line and polygon MVT MoveTo must have count one"));
                    }
                    if !points.is_empty() {
                        if kind == 3 {
                            return Err(invalid("MVT polygon ring lacks ClosePath"));
                        }
                        push(&mut paths, &mut points, false, extent, tile)?;
                    }
                } else if points.is_empty() {
                    return Err(invalid("MVT LineTo precedes MoveTo"));
                }
                *vertices = vertices.saturating_add(count);
                limit("map vertices", *vertices, maximum)?;
                for _ in 0..count {
                    for component in &mut cursor {
                        *component = component
                            .checked_add(zigzag(reader.varint()?))
                            .ok_or_else(|| invalid("MVT coordinate overflows"))?;
                        if *component < -i64::from(extent) * 8 || *component > i64::from(extent) * 9
                        {
                            return Err(invalid("MVT geometry exceeds its bounded tile buffer"));
                        }
                    }
                    points.push(cursor);
                }
            }
            7 if kind == 3 && count == 1 && points.len() >= 3 => {
                push(&mut paths, &mut points, true, extent, tile)?;
            }
            _ => return Err(invalid("MVT geometry contains an invalid command sequence")),
        }
    }
    if !points.is_empty() {
        if kind == 3 {
            return Err(invalid("MVT polygon ring is not closed"));
        }
        push(&mut paths, &mut points, false, extent, tile)?;
    }
    Ok(paths)
}

fn push(
    paths: &mut Vec<Path>,
    points: &mut Vec<[i64; 2]>,
    polygon: bool,
    extent: u32,
    tile: &TileRange,
) -> ImportResult<()> {
    if points.len() < if polygon { 3 } else { 2 } {
        return Err(invalid("MVT path has too few points"));
    }
    let exterior = if polygon {
        let area = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .map(|(a, b)| i128::from(a[0]) * i128::from(b[1]) - i128::from(b[0]) * i128::from(a[1]))
            .sum::<i128>();
        if area == 0 {
            return Err(invalid("MVT polygon ring has zero area"));
        }
        // Tile coordinates point downwards: clockwise exterior rings have positive area.
        area > 0
    } else {
        false
    };
    let mut coordinates = points
        .iter()
        .map(|p| unproject(*p, extent, tile))
        .collect::<Vec<_>>();
    if polygon && coordinates.first() != coordinates.last() {
        coordinates.push(coordinates[0]);
    }
    if coordinates.iter().flatten().any(|v| !v.is_finite())
        || coordinates
            .iter()
            .any(|p| p[0].abs() > 85.051129 || p[1].abs() > 180.0)
    {
        return Err(invalid("MVT geometry crosses supported Mercator bounds"));
    }
    paths.push(Path {
        coordinates,
        exterior,
    });
    points.clear();
    Ok(())
}

fn unproject(point: [i64; 2], extent: u32, tile: &TileRange) -> [f64; 2] {
    let n = f64::from(1u32 << tile.z);
    let x = (f64::from(tile.x) + point[0] as f64 / f64::from(extent)) / n;
    let y = (f64::from(tile.y) + point[1] as f64 / f64::from(extent)) / n;
    [
        ((1.0 - 2.0 * y) * std::f64::consts::PI)
            .sinh()
            .atan()
            .to_degrees(),
        x * 360.0 - 180.0,
    ]
}
