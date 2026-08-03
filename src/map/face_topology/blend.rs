//! Deterministic fixed-point blend of deformation components.
//!
//! `profiles::profile_displacement` combines its correlated and local
//! components here: the profile weights decide the *direction* (normalized in
//! Q24), while the *magnitude* comes from the stronger component, so
//! anti-parallel components can no longer cancel each other out.

const Q16: i64 = 65_536;

/// A 2D vector stored as fixed-point Q16 components (1.0 == `65_536`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedVectorQ16 {
    pub x: i64,
    pub y: i64,
}

/// Deterministic floor integer square root (Newton, from above).
///
/// Used for fixed-point vector normalization so the blend is exactly
/// platform-independent (no floating point before the final `Vec2`).
#[must_use]
pub fn integer_sqrt(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut estimate = value;
    let mut next = estimate.div_ceil(2);
    while next < estimate {
        estimate = next;
        next = estimate.midpoint(value / estimate);
    }
    estimate
}

/// Q16 magnitude of a fixed-point vector (`isqrt(x^2 + y^2)`).
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn component_length_q16(vector: FixedVectorQ16) -> i64 {
    #[allow(clippy::cast_sign_loss)]
    let squared = (vector.x * vector.x + vector.y * vector.y) as u64;
    integer_sqrt(squared) as i64
}

/// Deterministic profile target magnitude: the larger component length.
///
/// Because the magnitude never comes from a vector *sum*, anti-parallel
/// components cannot shrink the result; per-corner magnitude variation is
/// preserved from the strongest component.
#[must_use]
pub fn target_magnitude_q16(correlated: FixedVectorQ16, local: FixedVectorQ16) -> i64 {
    component_length_q16(correlated).max(component_length_q16(local))
}

/// Blends two components into a Q16 displacement vector.
///
/// The direction is the profile-weighted vector sum normalized in Q24 (the
/// local direction is used when the sum is exactly zero, which cannot happen
/// with the profile magnitude ranges in practice); the magnitude is
/// [`target_magnitude_q16`]. All arithmetic is integer and deterministic.
#[must_use]
pub fn blend_to_displacement_q16(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
) -> FixedVectorQ16 {
    const DIRECTION_SHIFT: i64 = 1 << 24;
    let wc = i64::from(correlated_weight_q16);
    let wl = i64::from(local_weight_q16);
    let bx = (correlated.x * wc + local.x * wl) / Q16;
    let by = (correlated.y * wc + local.y * wl) / Q16;
    let (dir_x, dir_y) = if bx == 0 && by == 0 {
        let length = component_length_q16(local);
        if length == 0 {
            (DIRECTION_SHIFT, 0)
        } else {
            (
                local.x * DIRECTION_SHIFT / length,
                local.y * DIRECTION_SHIFT / length,
            )
        }
    } else {
        #[allow(clippy::cast_possible_wrap)]
        let length = {
            #[allow(clippy::cast_sign_loss)]
            let squared = (bx * bx + by * by) as u64;
            integer_sqrt(squared) as i64
        };
        (bx * DIRECTION_SHIFT / length, by * DIRECTION_SHIFT / length)
    };
    let magnitude = target_magnitude_q16(correlated, local);
    FixedVectorQ16 {
        x: dir_x * magnitude / DIRECTION_SHIFT,
        y: dir_y * magnitude / DIRECTION_SHIFT,
    }
}
