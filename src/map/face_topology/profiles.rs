//! Experimental deterministic deformation profiles for diagnostic topology.
use crate::map::face_topology::blend::{
    blend_to_displacement_q16_with_policy, BlendReliabilityPolicy,
    PRODUCTION_BLEND_RELIABILITY_POLICY,
};
use crate::map::face_topology::corner_key::{
    corner_displacement, seed_for_corner, DISPLACEMENT_DIRECTIONS_Q15,
};
use crate::map::face_topology::types::SharedCornerKey;
use bevy::prelude::{Reflect, Vec2};

pub use crate::map::face_topology::blend::FixedVectorQ16;

const Q16: i64 = 65_536;
const Q15: i64 = 32_767;
const FIELD_DIRECTION_MASK: u64 = 0x0f;
const FIELD_STRENGTH_SHIFT: u32 = 12;
const FIELD_STRENGTH_MASK: u64 = 0xff;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord, Reflect)]
pub enum HexDeformationProfile {
    #[default]
    Subtle,
    Organic,
    PagoniaLike,
}

impl HexDeformationProfile {
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Subtle => Self::Organic,
            Self::Organic => Self::PagoniaLike,
            Self::PagoniaLike => Self::Subtle,
        }
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Subtle => "Subtle",
            Self::Organic => "Organic",
            Self::PagoniaLike => "PagoniaLike",
        }
    }

    #[must_use]
    pub const fn config(self) -> HexDeformationConfig {
        match self {
            Self::Subtle => HexDeformationConfig {
                component_magnitude_min_q16: 5_243,
                component_magnitude_max_q16: 7_864,
                absolute_displacement_cap_q16: 10_486,
                correlated_weight_q16: 0,
                local_weight_q16: 65_536,
                macro_span_hexes: 0,
                discriminator: 0x5355_4254,
            },
            Self::Organic => HexDeformationConfig {
                component_magnitude_min_q16: 7_864,
                component_magnitude_max_q16: 11_796,
                absolute_displacement_cap_q16: 14_418,
                correlated_weight_q16: 42_598,
                local_weight_q16: 22_938,
                macro_span_hexes: 5,
                discriminator: 0x4f52_4741,
            },
            Self::PagoniaLike => HexDeformationConfig {
                component_magnitude_min_q16: 10_486,
                component_magnitude_max_q16: 15_729,
                absolute_displacement_cap_q16: 18_350,
                correlated_weight_q16: 49_152,
                local_weight_q16: 16_384,
                macro_span_hexes: 5,
                discriminator: 0x5041_474f,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexDeformationConfig {
    /// Magnitude range used to build local and correlated components (Q16).
    /// This is an *input* range, not a final displacement guarantee.
    pub component_magnitude_min_q16: u16,
    /// Magnitude range used to build local and correlated components (Q16).
    pub component_magnitude_max_q16: u16,
    /// Absolute final-displacement cap as a Q16 ratio of the hex radius.
    pub absolute_displacement_cap_q16: u16,
    pub correlated_weight_q16: u32,
    pub local_weight_q16: u32,
    pub macro_span_hexes: i32,
    pub discriminator: u32,
}

impl HexDeformationConfig {
    /// Ratio (toward hex radius) that the final displacement is capped at.
    #[must_use]
    pub const fn absolute_displacement_cap_ratio(self) -> f32 {
        self.absolute_displacement_cap_q16 as f32 / Q16 as f32
    }

    /// Correlated component magnitude range ratio (input range, not guarantee).
    #[must_use]
    pub fn component_magnitude_min_ratio(self) -> f32 {
        f32::from(self.component_magnitude_min_q16) / Q16 as f32
    }

    /// Correlated component magnitude max ratio (input range, not guarantee).
    #[must_use]
    pub fn component_magnitude_max_ratio(self) -> f32 {
        f32::from(self.component_magnitude_max_q16) / Q16 as f32
    }
}

#[allow(clippy::cast_sign_loss)]
fn coordinate_bits(value: i64) -> u64 {
    value as u64
}

fn mix_word(state: u64, word: u64) -> u64 {
    let mut mixed = state.wrapping_add(word).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    mixed ^ (mixed >> 31)
}

fn field_node_hash(seed: u32, profile: HexDeformationProfile, q: i64, r: i64) -> u64 {
    let config = profile.config();
    let mut state = u64::from(seed) ^ u64::from(config.discriminator);
    state = mix_word(state, coordinate_bits(q));
    mix_word(state, coordinate_bits(r))
}

fn quantized_magnitude(hash: u64, config: HexDeformationConfig) -> i64 {
    let sample = u16::from(((hash >> FIELD_STRENGTH_SHIFT) & FIELD_STRENGTH_MASK) as u8);
    let span = u32::from(config.component_magnitude_max_q16 - config.component_magnitude_min_q16);
    let delta = u16::try_from(u32::from(sample) * span / u32::from(u8::MAX)).unwrap_or_default();
    i64::from(config.component_magnitude_min_q16 + delta)
        .min(i64::from(config.absolute_displacement_cap_q16))
}

fn vector_from_hash(hash: u64, config: HexDeformationConfig) -> FixedVectorQ16 {
    let direction_index =
        usize::from(u8::try_from(hash & FIELD_DIRECTION_MASK).unwrap_or_default());
    let direction = DISPLACEMENT_DIRECTIONS_Q15[direction_index];
    let magnitude = quantized_magnitude(hash, config);
    FixedVectorQ16 {
        x: i64::from(direction.0) * magnitude / Q15,
        y: i64::from(direction.1) * magnitude / Q15,
    }
}

/// Returns the fixed vector stored at an integer macro-field node.
#[must_use]
pub fn macro_field_node_vector(
    seed: u32,
    profile: HexDeformationProfile,
    macro_q: i64,
    macro_r: i64,
) -> FixedVectorQ16 {
    if profile == HexDeformationProfile::Subtle {
        return FixedVectorQ16 { x: 0, y: 0 };
    }
    vector_from_hash(
        field_node_hash(seed, profile, macro_q, macro_r),
        profile.config(),
    )
}

fn floor_div(value: i64, divisor: i64) -> i64 {
    let quotient = value / divisor;
    let remainder = value % divisor;
    if remainder != 0 && (remainder < 0) != (divisor < 0) {
        quotient - 1
    } else {
        quotient
    }
}

/// Converts a canonical corner to a Q16 axial field position.
#[must_use]
pub fn field_coordinate_q16(key: SharedCornerKey) -> (i64, i64) {
    let q_sum = i64::from(key.first().q) + i64::from(key.second().q) + i64::from(key.third().q);
    let r_sum = i64::from(key.first().r) + i64::from(key.second().r) + i64::from(key.third().r);
    (floor_div(q_sum * Q16, 3), floor_div(r_sum * Q16, 3))
}

fn interpolate_axis(a: i64, b: i64, fraction_q16: i64) -> i64 {
    (a * (Q16 - fraction_q16) + b * fraction_q16) / Q16
}

/// Bilinearly interpolates the deterministic coarse field at a shared corner.
#[must_use]
pub fn interpolated_correlated_field(
    seed: u32,
    key: SharedCornerKey,
    profile: HexDeformationProfile,
) -> FixedVectorQ16 {
    let config = profile.config();
    if config.macro_span_hexes == 0 {
        return FixedVectorQ16 { x: 0, y: 0 };
    }
    let (q_q16, r_q16) = field_coordinate_q16(key);
    let span = i64::from(config.macro_span_hexes) * Q16;
    let macro_q = floor_div(q_q16, span);
    let macro_r = floor_div(r_q16, span);
    let fraction_q = floor_div((q_q16 - macro_q * span) * Q16, span);
    let fraction_r = floor_div((r_q16 - macro_r * span) * Q16, span);
    let v00 = macro_field_node_vector(seed, profile, macro_q, macro_r);
    let v10 = macro_field_node_vector(seed, profile, macro_q + 1, macro_r);
    let v01 = macro_field_node_vector(seed, profile, macro_q, macro_r + 1);
    let v11 = macro_field_node_vector(seed, profile, macro_q + 1, macro_r + 1);
    let x0 = interpolate_axis(v00.x, v10.x, fraction_q);
    let x1 = interpolate_axis(v01.x, v11.x, fraction_q);
    let y0 = interpolate_axis(v00.y, v10.y, fraction_q);
    let y1 = interpolate_axis(v01.y, v11.y, fraction_q);
    FixedVectorQ16 {
        x: interpolate_axis(x0, x1, fraction_r),
        y: interpolate_axis(y0, y1, fraction_r),
    }
}

/// Returns the profile's deterministic local high-frequency component.
#[must_use]
pub fn local_component_q16(
    seed: u32,
    key: SharedCornerKey,
    profile: HexDeformationProfile,
) -> FixedVectorQ16 {
    let config = profile.config();
    let hash = seed_for_corner(seed ^ config.discriminator, key);
    vector_from_hash(hash, config)
}

/// Combines the correlated and local components, preserving Subtle exactly.
///
/// `Subtle` keeps the legacy `corner_displacement` path bit-for-bit. `Organic`
/// and `PagoniaLike` use the deterministic magnitude-preserving blend from
/// `blend`: weighted direction at the stronger component magnitude, so
/// anti-parallel components cannot cancel the result.
#[must_use]
pub fn profile_displacement(
    seed: u32,
    key: SharedCornerKey,
    radius: f32,
    profile: HexDeformationProfile,
) -> Vec2 {
    profile_displacement_with_policy(
        seed,
        key,
        radius,
        profile,
        PRODUCTION_BLEND_RELIABILITY_POLICY,
    )
}

/// Like [`profile_displacement`], but under an explicit blend policy.
///
/// `Subtle` has no blend and keeps the legacy `corner_displacement` path
/// regardless of the policy; the policy only shapes `Organic`/`PagoniaLike`.
#[must_use]
pub fn profile_displacement_with_policy(
    seed: u32,
    key: SharedCornerKey,
    radius: f32,
    profile: HexDeformationProfile,
    policy: BlendReliabilityPolicy,
) -> Vec2 {
    if profile == HexDeformationProfile::Subtle {
        return corner_displacement(seed, key, radius);
    }
    let config = profile.config();
    let correlated = interpolated_correlated_field(seed, key, profile);
    let local = local_component_q16(seed, key, profile);
    let blended = blend_to_displacement_q16_with_policy(
        correlated,
        local,
        config.correlated_weight_q16,
        config.local_weight_q16,
        policy,
    );
    Vec2::new(
        radius * blended.x as f32 / Q16 as f32,
        radius * blended.y as f32 / Q16 as f32,
    )
}
