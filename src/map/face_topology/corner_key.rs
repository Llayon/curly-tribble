//! Canonical corner keys and portable displacement determinism for hex topology.
//!
//! The persistent displacement contract is scoped to `HexFaceTopology`: integer
//! axial coordinates feed a fixed wrapping mixer, low hash bits select an
//! immutable Q15 direction, and other hash bits select a Q16 magnitude. The
//! generator sorts map keys before construction, so `HashMap` iteration order does
//! not affect geometry. No random-number algorithm or runtime trigonometry is
//! involved in corner displacement; its f32 bit patterns are golden-tested.
use crate::map::data::HEX_SIZE;
use crate::map::face_topology::types::{HexFaceTopologyError, SharedCornerKey};
use crate::map::HexCoord;
use bevy::prelude::Vec2;

const DIRECTION_INDEX_MASK: u8 = 0x0f;
const MAGNITUDE_SHIFT: u32 = 8;
const MAGNITUDE_MASK: u64 = 0xff;
const Q16_ONE: f32 = 65_536.0;
const Q15_ONE: f32 = 32_767.0;
const MIN_MAGNITUDE_Q16: u16 = 5_243;
const MAX_MAGNITUDE_Q16: u16 = 7_864;
const MAX_DISPLACEMENT_Q16: u16 = 10_486;

/// Sixteen immutable Q15 unit directions, spaced at 22.5 degree intervals.
pub const DISPLACEMENT_DIRECTIONS_Q15: [(i16, i16); 16] = [
    (32_767, 0),
    (30_274, 12_539),
    (23_170, 23_170),
    (12_539, 30_274),
    (0, 32_767),
    (-12_539, 30_274),
    (-23_170, 23_170),
    (-30_274, 12_539),
    (-32_767, 0),
    (-30_274, -12_539),
    (-23_170, -23_170),
    (-12_539, -30_274),
    (0, -32_767),
    (12_539, -30_274),
    (23_170, -23_170),
    (30_274, -12_539),
];

/// Applies one fixed SplitMix64-style round with wrapping u64 arithmetic.
#[must_use]
fn splitmix64_round(state: u64, word: u64) -> u64 {
    let mut mixed = state.wrapping_add(word).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^ (mixed >> 31)
}

#[allow(clippy::cast_sign_loss)]
fn coordinate_bits(value: i32) -> u64 {
    // Rust's signed-to-unsigned cast is modulo 2^32 and is endian-independent.
    u64::from(value as u32)
}

/// Computes a stable, deterministic 64-bit seed for a corner.
///
/// Six coordinate words are mixed in `(q, r)` order for each canonical key cell.
/// The fixed constants and explicit u32-to-u64 conversion make this independent
/// of Rust hashers, platform endianness, and `usize` width.
#[must_use]
pub fn seed_for_corner(seed_val: u32, key: SharedCornerKey) -> u64 {
    let c0 = key.first();
    let c1 = key.second();
    let c2 = key.third();

    // Convert i32 to u32 first (preserving bit pattern), then to u64
    let inputs: [u64; 6] = [
        coordinate_bits(c0.q),
        coordinate_bits(c0.r),
        coordinate_bits(c1.q),
        coordinate_bits(c1.r),
        coordinate_bits(c2.q),
        coordinate_bits(c2.r),
    ];

    let mut h = u64::from(seed_val);
    for val in inputs {
        h = splitmix64_round(h, val);
    }

    h
}

/// Derives a portable corner displacement from a fixed mixed corner seed.
///
/// Bits 0..=3 select one of the sixteen Q15 directions. Bits 8..=15 select
/// one of 256 Q16 magnitudes spanning 8% through 12% of `radius`; the 16%
/// Q16 cap is retained as an explicit upper bound. Floating-point conversion
/// occurs only when constructing the final `Vec2`.
#[must_use]
pub fn corner_displacement(seed_val: u32, key: SharedCornerKey, radius: f32) -> Vec2 {
    let mixed = seed_for_corner(seed_val, key);
    let direction_index =
        usize::from(u8::try_from(mixed & u64::from(DIRECTION_INDEX_MASK)).unwrap_or_default());
    let direction = DISPLACEMENT_DIRECTIONS_Q15[direction_index];
    let magnitude_sample = u16::from(((mixed >> MAGNITUDE_SHIFT) & MAGNITUDE_MASK) as u8);
    let magnitude_span = u32::from(MAX_MAGNITUDE_Q16 - MIN_MAGNITUDE_Q16);
    let magnitude_delta =
        u16::try_from(u32::from(magnitude_sample) * magnitude_span / u32::from(u8::MAX))
            .unwrap_or_default();
    let magnitude_q16 = MIN_MAGNITUDE_Q16 + magnitude_delta;
    let magnitude_q16 = magnitude_q16.min(MAX_DISPLACEMENT_Q16);

    Vec2::new(
        radius * (f32::from(magnitude_q16) / Q16_ONE) * (f32::from(direction.0) / Q15_ONE),
        radius * (f32::from(magnitude_q16) / Q16_ONE) * (f32::from(direction.1) / Q15_ONE),
    )
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
