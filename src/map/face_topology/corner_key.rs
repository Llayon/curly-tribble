// src/map/face_topology/corner_key.rs
use crate::map::data::HEX_SIZE;
use crate::map::face_topology::types::SharedCornerKey;
use crate::map::HexCoord;
use bevy::prelude::*;

pub struct CornerKeyPlugin;

impl Plugin for CornerKeyPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Derives the canonical `SharedCornerKey` for a corner index (0..6) of a `HexCoord`.
///
/// Each corner of a flat/pointy hex is shared by up to 3 adjacent hexes on the infinite lattice.
/// Sorting these 3 meeting hex coordinates lexicographically by (q, r) produces a unique,
/// invariant `SharedCornerKey` regardless of which incident cell is processing it.
#[must_use]
pub fn canonical_corner_key(coord: HexCoord, corner_idx: usize) -> SharedCornerKey {
    let neighbors = coord.neighbors();
    let idx = corner_idx % 6;
    let (h_b, h_c) = match idx {
        0 => (neighbors[0], neighbors[5]), // +X, +Z: (q+1, r), (q, r+1)
        1 => (neighbors[5], neighbors[4]), // +Z: (q, r+1), (q-1, r+1)
        2 => (neighbors[4], neighbors[3]), // -X, +Z: (q-1, r+1), (q-1, r)
        3 => (neighbors[3], neighbors[2]), // -X, -Z: (q-1, r), (q, r-1)
        4 => (neighbors[2], neighbors[1]), // -Z: (q, r-1), (q+1, r-1)
        _ => (neighbors[1], neighbors[0]), // +X, -Z: (q+1, r-1), (q+1, r)
    };

    let mut triplet = [coord, h_b, h_c];
    triplet.sort_by_key(|c| (c.q, c.r));
    SharedCornerKey::new(triplet[0], triplet[1], triplet[2])
}

/// Computes the un-displaced 2D (X/Z) world position of a regular hex corner.
#[must_use]
pub fn regular_corner_position(key: SharedCornerKey) -> Vec2 {
    let anchor = key.first();
    let center_3d = anchor.to_world(HEX_SIZE);
    let center = Vec2::new(center_3d.x, center_3d.z);

    // Find which corner index of anchor corresponds to `key`
    for i in 0..6 {
        if canonical_corner_key(anchor, i) == key {
            #[allow(clippy::cast_precision_loss)]
            let angle_deg = 60.0 * i as f32 + 30.0;
            let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
            return Vec2::new(
                center.x + HEX_SIZE * angle_rad.cos(),
                center.y + HEX_SIZE * angle_rad.sin(),
            );
        }
    }

    center
}
