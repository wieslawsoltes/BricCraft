use serde::{Deserialize, Serialize};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Shape {
    Brick = 0,
    Plate = 1,
    Tile = 2,
    Slope = 3,
    Round = 4,
    Wheel = 5,
    Gear = 6,
    Axle = 7,
    Beam = 8,
    Window = 9,
    Door = 10,
    Rail = 11,
    Wing = 12,
    Cone = 13,
    Arch = 14,
    Propeller = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Category {
    Bricks,
    Plates,
    Tiles,
    Slopes,
    Round,
    Architecture,
    Technic,
    Wheels,
    Transport,
    Nature,
    Furniture,
    Power,
}

impl Category {
    pub const ALL: [Self; 12] = [
        Self::Bricks,
        Self::Plates,
        Self::Tiles,
        Self::Slopes,
        Self::Round,
        Self::Architecture,
        Self::Technic,
        Self::Wheels,
        Self::Transport,
        Self::Nature,
        Self::Furniture,
        Self::Power,
    ];
    pub const fn name(self) -> &'static str {
        match self {
            Self::Bricks => "Bricks",
            Self::Plates => "Plates",
            Self::Tiles => "Tiles",
            Self::Slopes => "Slopes",
            Self::Round => "Round",
            Self::Architecture => "Architecture",
            Self::Technic => "Technic",
            Self::Wheels => "Wheels",
            Self::Transport => "Transport",
            Self::Nature => "Nature",
            Self::Furniture => "Furniture",
            Self::Power => "Power",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Part {
    pub id: u16,
    pub name: &'static str,
    pub category: Category,
    /// X studs, Y plates, Z studs.
    pub size: [u8; 3],
    pub shape: Shape,
}

impl Part {
    pub const fn rotated_size(&self, rotation: u8) -> [u8; 3] {
        if rotation & 1 == 0 {
            self.size
        } else {
            [self.size[2], self.size[1], self.size[0]]
        }
    }

    pub const fn has_studs(&self) -> bool {
        matches!(
            self.shape,
            Shape::Brick | Shape::Plate | Shape::Round | Shape::Arch | Shape::Window | Shape::Door
        )
    }

    /// Approximate ABS shell mass, in kilograms, using the dimensional envelope.
    pub fn mass(&self) -> f32 {
        let [x, y, z] = self.size.map(f32::from);
        let shell = (2.0 * (x * z + (x + z) * y * 0.4) * 0.15).max(0.3);
        shell * 0.00054
    }
}
