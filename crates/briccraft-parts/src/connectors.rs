use crate::{Part, Shape};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectorKind {
    Stud,
    Socket,
    Pin,
    PinHole,
    Axle,
    AxleHole,
    Hitch,
}

impl ConnectorKind {
    pub const fn mates(self, other: Self) -> bool {
        matches!(
            (self, other),
            (Self::Stud, Self::Socket)
                | (Self::Socket, Self::Stud)
                | (Self::Pin, Self::PinHole)
                | (Self::PinHole, Self::Pin)
                | (Self::Axle, Self::AxleHole)
                | (Self::AxleHole, Self::Axle)
                | (Self::Hitch, Self::Hitch)
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Connector {
    pub kind: ConnectorKind,
    pub position: [f32; 3],
    pub axis: [f32; 3],
}

impl Part {
    /// Connector positions in the unrotated part's local stud coordinates.
    pub fn connectors(&self) -> Vec<Connector> {
        let mut out = Vec::new();
        let [x, y, z] = self.size;
        if self.has_studs() || matches!(self.shape, Shape::Tile | Shape::Slope | Shape::Wing) {
            for ix in 0..x {
                for iz in 0..z {
                    let position = [ix as f32 + 0.5, 0.0, iz as f32 + 0.5];
                    out.push(Connector {
                        kind: ConnectorKind::Socket,
                        position,
                        axis: [0.0, -1.0, 0.0],
                    });
                    if self.has_studs() {
                        out.push(Connector {
                            kind: ConnectorKind::Stud,
                            position: [position[0], y as f32 * 0.4, position[2]],
                            axis: [0.0, 1.0, 0.0],
                        });
                    }
                }
            }
        }
        match self.shape {
            Shape::Beam => {
                for iz in 0..z {
                    for side in [0.0, x as f32] {
                        out.push(Connector {
                            kind: ConnectorKind::PinHole,
                            position: [side, y as f32 * 0.2, iz as f32 + 0.5],
                            axis: [if side == 0.0 { -1.0 } else { 1.0 }, 0.0, 0.0],
                        });
                    }
                }
            }
            Shape::Axle => {
                for iz in [0.0, z as f32] {
                    out.push(Connector {
                        kind: ConnectorKind::Axle,
                        position: [x as f32 * 0.5, y as f32 * 0.2, iz],
                        axis: [0.0, 0.0, if iz == 0.0 { -1.0 } else { 1.0 }],
                    });
                }
            }
            Shape::Gear | Shape::Wheel | Shape::Propeller => {
                out.push(Connector {
                    kind: ConnectorKind::AxleHole,
                    position: [x as f32 * 0.5, y as f32 * 0.2, z as f32 * 0.5],
                    axis: [1.0, 0.0, 0.0],
                });
            }
            _ => {}
        }
        out
    }
}
