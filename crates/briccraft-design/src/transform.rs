use crate::{ConstructionDesign, DesignError};
use briccraft_world::Placement;

impl ConstructionDesign {
    pub fn rotated_dimensions(&self, rotation: u8) -> Result<[u8; 3], DesignError> {
        if rotation > 3 {
            return Err(DesignError::InvalidRotation(rotation));
        }
        let [x, y, z] = self.dimensions;
        Ok(if rotation & 1 == 0 {
            [x, y, z]
        } else {
            [z, y, x]
        })
    }

    /// The destination is the minimum corner of the rotated envelope, in X studs/Y plates/Z studs.
    /// Quarter-turns use the same positive-yaw convention as built-in world blueprints.
    pub fn placements(
        &self,
        origin: [i32; 3],
        rotation: u8,
    ) -> Result<Vec<Placement>, DesignError> {
        self.validate()?;
        self.rotated_dimensions(rotation)?;
        let [width, _, depth] = self.dimensions.map(i32::from);
        self.parts
            .iter()
            .map(|source| {
                let [x, y, z] = source.position;
                let size = source.size().map(i32::from);
                let rotated_size = if rotation & 1 == 0 {
                    size
                } else {
                    [size[2], size[1], size[0]]
                };
                let local = match rotation {
                    0 => [x, y, z],
                    1 => [z, y, width - x - size[0]],
                    2 => [width - x - size[0], y, depth - z - size[2]],
                    _ => [depth - z - size[2], y, x],
                };
                let mut position = [0; 3];
                for axis in 0..3 {
                    position[axis] = origin[axis]
                        .checked_add(local[axis])
                        .ok_or(DesignError::CoordinateOverflow)?;
                    position[axis]
                        .checked_add(rotated_size[axis])
                        .ok_or(DesignError::CoordinateOverflow)?;
                }
                Ok(Placement::new(
                    source.part_id,
                    source.color,
                    position,
                    (source.rotation + rotation) & 3,
                ))
            })
            .collect()
    }
}
