/// Canonical corner key derivation and stable seed mixing for hex face topology.
use crate::map::data::HEX_SIZE;
use crate::map::face_topology::types::{HexFaceTopologyError, SharedCornerKey};
use crate::map::HexCoord;
use bevy::prelude::*;

pub struct CornerKeyPlugin;
impl Plugin for CornerKeyPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Computes a stable, deterministic 64-bit seed for a corner.
///
/// Uses explicit wrapping 64-bit integer arithmetic (`SplitMix64` finalizer)
/// depending strictly on `WorldSeed` and `SharedCornerKey` coordinates `(c0, c1, c2)`.
/// This guarantees bit-identical results across Rust compiler versions, platforms,
/// and endianness.
#[must_use]
#[allow(clippy::cast_sign_loss)]
pub fn seed_for_corner(seed_val: u32, key: SharedCornerKey) -> u64 {
    let c0 = key.first();
    let c1 = key.second();
    let c2 = key.third();

    // Convert i32 to u32 first (preserving bit pattern), then to u64
    let inputs: [u64; 6] = [
        u64::from(c0.q as u32),
        u64::from(c0.r as u32),
        u64::from(c1.q as u32),
        u64::from(c1.r as u32),
        u64::from(c2.q as u32),
        u64::from(c2.r as u32),
    ];

    let mut h = u64::from(seed_val);
    for val in inputs {
        h = h.wrapping_add(val).wrapping_mul(0x9e37_79b9_7f4a_7c15);
        h = (h ^ (h >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        h = (h ^ (h >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        h ^= h >> 31;
    }

    h
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
///
/// # Errors
/// Returns `HexFaceTopologyError::CornerKeyMismatch` if `key` does not match any corner
/// of its anchor cell.
pub fn regular_corner_position(key: SharedCornerKey) -> Result<Vec2, HexFaceTopologyError> {
    let anchor = key.first();
    let center_3d = anchor.to_world(HEX_SIZE);
    let center = Vec2::new(center_3d.x, center_3d.z);

    for i in 0..6 {
        if canonical_corner_key(anchor, i) == key {
            #[allow(clippy::cast_precision_loss)]
            let angle_deg = 60.0 * i as f32 + 30.0;
            let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
            return Ok(Vec2::new(
                center.x + HEX_SIZE * angle_rad.cos(),
                center.y + HEX_SIZE * angle_rad.sin(),
            ));
        }
    }

    Err(HexFaceTopologyError::CornerKeyMismatch(key))
}
