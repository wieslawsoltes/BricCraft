use crate::{CHUNK_SIZE, StreamError, WINDOW_CHUNKS};
use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ChunkCoord {
    pub x: i64,
    pub z: i64,
}

impl ChunkCoord {
    pub const fn new(x: i64, z: i64) -> Self {
        Self { x, z }
    }
    pub fn containing(x: i64, z: i64) -> Self {
        Self {
            x: x.div_euclid(CHUNK_SIZE),
            z: z.div_euclid(CHUNK_SIZE),
        }
    }
    pub fn origin(self) -> Result<WindowOrigin, StreamError> {
        let x = self
            .x
            .checked_mul(CHUNK_SIZE)
            .ok_or(StreamError::CoordinateOverflow)?;
        let z = self
            .z
            .checked_mul(CHUNK_SIZE)
            .ok_or(StreamError::CoordinateOverflow)?;
        Ok(WindowOrigin { x, z })
    }
}

/// X/Z are exact signed global studs; Y is an absolute plate coordinate.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct GlobalPos {
    pub x: i64,
    pub y: i32,
    pub z: i64,
}

impl GlobalPos {
    pub const fn new(x: i64, y: i32, z: i64) -> Self {
        Self { x, y, z }
    }
    pub fn chunk(self) -> ChunkCoord {
        ChunkCoord::containing(self.x, self.z)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowOrigin {
    pub x: i64,
    pub z: i64,
}

impl WindowOrigin {
    pub const fn new(x: i64, z: i64) -> Self {
        Self { x, z }
    }

    pub fn validate(self) -> Result<(), StreamError> {
        if self.x.rem_euclid(CHUNK_SIZE) != 0 || self.z.rem_euclid(CHUNK_SIZE) != 0 {
            return Err(StreamError::UnalignedOrigin(self));
        }
        self.x
            .checked_add(96)
            .ok_or(StreamError::CoordinateOverflow)?;
        self.z
            .checked_add(96)
            .ok_or(StreamError::CoordinateOverflow)?;
        Ok(())
    }

    pub fn around(position: GlobalPos) -> Result<Self, StreamError> {
        let x = position
            .x
            .checked_sub(48)
            .ok_or(StreamError::CoordinateOverflow)?;
        let z = position
            .z
            .checked_sub(48)
            .ok_or(StreamError::CoordinateOverflow)?;
        let origin = ChunkCoord::containing(x, z).origin()?;
        origin.validate()?;
        Ok(origin)
    }

    pub fn global_cell(self, local: [i32; 3]) -> Result<GlobalPos, StreamError> {
        Ok(GlobalPos {
            x: self
                .x
                .checked_add(i64::from(local[0]))
                .ok_or(StreamError::CoordinateOverflow)?,
            y: local[1],
            z: self
                .z
                .checked_add(i64::from(local[2]))
                .ok_or(StreamError::CoordinateOverflow)?,
        })
    }

    /// Subtract before converting to float, retaining precision even far beyond f64's integer range.
    pub fn local_position(self, global: GlobalPos) -> Result<[f32; 3], StreamError> {
        Ok([
            global
                .x
                .checked_sub(self.x)
                .ok_or(StreamError::CoordinateOverflow)? as f32,
            global.y as f32 * 0.4,
            global
                .z
                .checked_sub(self.z)
                .ok_or(StreamError::CoordinateOverflow)? as f32,
        ])
    }

    /// Human-readable approximate world units. Use `global_cell` for persistent exact coordinates.
    pub fn global_position(self, local: [f32; 3]) -> [f64; 3] {
        [
            self.x as f64 + f64::from(local[0]),
            f64::from(local[1]),
            self.z as f64 + f64::from(local[2]),
        ]
    }

    pub fn chunks(self) -> Result<Vec<ChunkCoord>, StreamError> {
        self.validate()?;
        let base = ChunkCoord::containing(self.x, self.z);
        let mut chunks = Vec::with_capacity(WINDOW_CHUNKS * WINDOW_CHUNKS);
        for z in 0..WINDOW_CHUNKS as i64 {
            for x in 0..WINDOW_CHUNKS as i64 {
                chunks.push(ChunkCoord::new(base.x + x, base.z + z));
            }
        }
        // Center-out generation makes pending progress useful for diagnostics without publishing holes.
        chunks.sort_by_key(|c| ((c.x - base.x) * 2 - 5).pow(2) + ((c.z - base.z) * 2 - 5).pow(2));
        Ok(chunks)
    }

    pub fn rebase_around(self, camera_local: [f32; 3]) -> Result<Option<Rebase>, StreamError> {
        if camera_local.iter().any(|v| !v.is_finite()) {
            return Err(StreamError::InvalidPosition);
        }
        if (32.0..=64.0).contains(&camera_local[0]) && (32.0..=64.0).contains(&camera_local[2]) {
            return Ok(None);
        }
        let shift_x = ((f64::from(camera_local[0]) - 48.0) / CHUNK_SIZE as f64).floor();
        let shift_z = ((f64::from(camera_local[2]) - 48.0) / CHUNK_SIZE as f64).floor();
        if shift_x.abs() > 1_000_000.0 || shift_z.abs() > 1_000_000.0 {
            return Err(StreamError::InvalidPosition);
        }
        let dx = (shift_x as i64) * CHUNK_SIZE;
        let dz = (shift_z as i64) * CHUNK_SIZE;
        let origin = Self {
            x: self
                .x
                .checked_add(dx)
                .ok_or(StreamError::CoordinateOverflow)?,
            z: self
                .z
                .checked_add(dz)
                .ok_or(StreamError::CoordinateOverflow)?,
        };
        origin.validate()?;
        Ok(Some(Rebase {
            origin,
            local_delta: [-dx as f32, 0.0, -dz as f32],
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rebase {
    pub origin: WindowOrigin,
    /// Add to camera, vehicle, and local preview world-unit positions when publishing the new window.
    pub local_delta: [f32; 3],
}

impl Rebase {
    pub fn translate(self, local: [f32; 3]) -> [f32; 3] {
        std::array::from_fn(|axis| local[axis] + self.local_delta[axis])
    }
}
