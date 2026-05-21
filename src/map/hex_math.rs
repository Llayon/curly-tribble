use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Default,
    Reflect,
    Serialize,
    Deserialize,
)]
pub struct HexCoord {
    pub q: i32,
    pub r: i32,
}

impl HexCoord {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    pub fn neighbors(&self) -> [Self; 6] {
        let q = self.q;
        let r = self.r;
        [
            Self::new(q + 1, r),
            Self::new(q + 1, r - 1),
            Self::new(q, r - 1),
            Self::new(q - 1, r),
            Self::new(q - 1, r + 1),
            Self::new(q, r + 1),
        ]
    }

    /// Convert axial coordinates to world 3D coordinates (x, 0, z)
    /// Pointy-topped hexes
    #[must_use]
    pub fn to_world(&self, size: f32) -> Vec3 {
        let x = size * (3.0f32.sqrt() * self.q as f32 + 3.0f32.sqrt() / 2.0 * self.r as f32);
        let z = size * (3.0 / 2.0 * self.r as f32);
        Vec3::new(x, 0.0, z)
    }

    #[must_use]
    pub fn from_world(world: Vec3, size: f32) -> Self {
        let q = (3.0f32.sqrt() / 3.0 * world.x - 1.0 / 3.0 * world.z) / size;
        let r = (2.0 / 3.0 * world.z) / size;
        Self::axial_round(q, r)
    }

    fn axial_round(q: f32, r: f32) -> Self {
        let s = -q - r;
        let mut rq = q.round();
        let mut rr = r.round();
        let rs = s.round();

        let q_diff = (rq - q).abs();
        let r_diff = (rr - r).abs();
        let s_diff = (rs - s).abs();

        if q_diff > r_diff && q_diff > s_diff {
            rq = -rr - rs;
        } else if r_diff > s_diff {
            rr = -rq - rs;
        }

        Self::new(rq as i32, rr as i32)
    }

    pub fn distance(&self, other: Self) -> i32 {
        ((self.q - other.q).abs()
            + (self.q + self.r - other.q - other.r).abs()
            + (self.r - other.r).abs()) / 2
    }
}
