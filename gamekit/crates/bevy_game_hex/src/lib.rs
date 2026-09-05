//! Pure axial coordinates and configurable 2D layout/picking.
//!
//! Boards, bounds, occupancy, pieces, movement legality, terrain, and victory
//! belong to games. This crate has no Bevy, rendering, or networking dependency.

/// Integer axial coordinate; the implicit cube coordinate is `-q-r`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hex {
    /// First axial coordinate.
    pub q: i32,
    /// Second axial coordinate.
    pub r: i32,
}

impl Hex {
    /// Origin of the axial lattice.
    pub const ZERO: Self = Self { q: 0, r: 0 };

    /// Exact lattice distance, widened to avoid overflow at coordinate extremes.
    #[must_use]
    pub fn distance(self, other: Self) -> u64 {
        let dq = i64::from(self.q) - i64::from(other.q);
        let dr = i64::from(self.r) - i64::from(other.r);
        dq.unsigned_abs()
            .max(dr.unsigned_abs())
            .max((dq + dr).unsigned_abs())
    }

    /// Adjacent coordinate, or `None` at the representable integer boundary.
    #[must_use]
    pub fn neighbor(self, direction: Direction) -> Option<Self> {
        let (q, r) = match direction {
            Direction::Q => (1, 0),
            Direction::QMinusR => (1, -1),
            Direction::MinusR => (0, -1),
            Direction::MinusQ => (-1, 0),
            Direction::RMinusQ => (-1, 1),
            Direction::R => (0, 1),
        };
        Some(Self {
            q: self.q.checked_add(q)?,
            r: self.r.checked_add(r)?,
        })
    }
}

/// Six lattice directions, independent of screen orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Positive q.
    Q,
    /// Positive q, negative r.
    QMinusR,
    /// Negative r.
    MinusR,
    /// Negative q.
    MinusQ,
    /// Negative q, positive r.
    RMinusQ,
    /// Positive r.
    R,
}

impl Direction {
    /// Deterministic order around a cell.
    pub const ALL: [Self; 6] = [
        Self::Q,
        Self::QMinusR,
        Self::MinusR,
        Self::MinusQ,
        Self::RMinusQ,
        Self::R,
    ];
}

/// Orientation of hexagons in the 2D plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// A vertex points along negative/positive y.
    Pointy,
    /// A vertex points along negative/positive x.
    Flat,
}

/// Validated regular-hex layout. Camera transforms remain game-owned.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexLayout {
    orientation: Orientation,
    radius: f64,
    origin: [f64; 2],
}

/// Invalid size, non-finite position, or unrepresentable coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeometryError;

impl std::fmt::Display for GeometryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("invalid or unrepresentable hex geometry")
    }
}
impl std::error::Error for GeometryError {}

impl HexLayout {
    /// Validates positive finite radius and finite origin coordinates.
    pub fn new(
        orientation: Orientation,
        radius: f64,
        origin: [f64; 2],
    ) -> Result<Self, GeometryError> {
        if !radius.is_finite() || radius <= 0.0 || origin.iter().any(|value| !value.is_finite()) {
            return Err(GeometryError);
        }
        Ok(Self {
            orientation,
            radius,
            origin,
        })
    }

    /// Cell center in the game's 2D plane, before camera transforms.
    pub fn center(self, hex: Hex) -> Result<[f64; 2], GeometryError> {
        let (q, r) = (f64::from(hex.q), f64::from(hex.r));
        let [ox, oy] = self.origin;
        let (x, y) = match self.orientation {
            Orientation::Pointy => (3.0_f64.sqrt() * (q + r / 2.0), 1.5 * r),
            Orientation::Flat => (1.5 * q, 3.0_f64.sqrt() * (r + q / 2.0)),
        };
        let center = [ox + x * self.radius, oy + y * self.radius];
        if center.iter().any(|value| !value.is_finite()) {
            return Err(GeometryError);
        }
        Ok(center)
    }

    /// Nearest cell for a point in the layout plane, not a board-membership test.
    /// Exact edge ties use deterministic cube rounding; games apply board bounds.
    pub fn pick(self, [x, y]: [f64; 2]) -> Result<Hex, GeometryError> {
        let [ox, oy] = self.origin;
        let (x, y) = ((x - ox) / self.radius, (y - oy) / self.radius);
        let (q, r) = match self.orientation {
            Orientation::Pointy => (x / 3.0_f64.sqrt() - y / 3.0, 2.0 * y / 3.0),
            Orientation::Flat => (2.0 * x / 3.0, y / 3.0_f64.sqrt() - x / 3.0),
        };
        let s = -q - r;
        let (mut rq, mut rr, rs) = (q.round(), r.round(), s.round());
        let (dq, dr, ds) = ((rq - q).abs(), (rr - r).abs(), (rs - s).abs());
        if dq > dr && dq > ds {
            rq = -rr - rs;
        } else if dr > ds {
            rr = -rq - rs;
        }
        if [rq, rr].iter().any(|value| {
            !value.is_finite() || *value < f64::from(i32::MIN) || *value > f64::from(i32::MAX)
        }) {
            return Err(GeometryError);
        }
        Ok(Hex {
            q: rq as i32,
            r: rr as i32,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighbors_are_distinct_and_one_step_away() {
        let neighbors =
            Direction::ALL.map(|direction| Hex::ZERO.neighbor(direction).expect("origin neighbor"));
        assert_eq!(
            neighbors
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            6
        );
        for neighbor in neighbors {
            assert_eq!(neighbor.distance(Hex::ZERO), 1);
        }
        assert_eq!(Hex { q: i32::MAX, r: 0 }.neighbor(Direction::Q), None);
        assert_eq!(
            Hex {
                q: i32::MAX,
                r: i32::MAX
            }
            .distance(Hex {
                q: i32::MIN,
                r: i32::MIN
            }),
            8_589_934_590
        );
    }

    #[test]
    fn both_orientations_round_trip_with_scaled_translated_origins() {
        for orientation in [Orientation::Pointy, Orientation::Flat] {
            let layout = HexLayout::new(orientation, 23.5, [125.0, -321.0]).expect("layout");
            for q in -16..=16 {
                for r in -16..=16 {
                    let hex = Hex { q, r };
                    assert_eq!(layout.pick(layout.center(hex).expect("center")), Ok(hex));
                }
            }
            assert!(layout.pick([f64::NAN, 0.0]).is_err());
            assert!(layout.pick([f64::MAX, f64::MAX]).is_err());
        }
        assert!(HexLayout::new(Orientation::Pointy, 0.0, [0.0, 0.0]).is_err());
    }
}
