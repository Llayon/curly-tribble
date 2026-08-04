//! Deterministic fixed-point blend of deformation components.
//!
//! `profiles::profile_displacement` combines its correlated and local
//! components here: the profile weights decide the *direction* (normalized in
//! Q24), while the *magnitude* comes from the stronger component, so
//! anti-parallel components can no longer cancel each other out.
//!
//! The pre-normalization weighted direction is exposed through
//! [`weighted_blend_diagnostics`] so tests can audit near-cancellation points
//! without recomputing the blend.

const Q16: i64 = 65_536;

/// A 2D vector stored as fixed-point Q16 components (1.0 == `65_536`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedVectorQ16 {
    pub x: i64,
    pub y: i64,
}

/// Pre-normalization intermediates of one weighted blend.
///
/// All values are fixed-point (Q16 unless noted) and computed with the exact
/// same integer arithmetic used by [`blend_to_displacement_q16`], so the
/// diagnostics can never disagree with the produced displacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightedBlendDiagnostics {
    /// `(correlated.x * wc + local.x * wl) / Q16` before normalization.
    pub weighted_x_q16: i64,
    /// `(correlated.y * wc + local.y * wl) / Q16` before normalization.
    pub weighted_y_q16: i64,
    /// `isqrt(weighted_x^2 + weighted_y^2)` in Q16 (0 when both are zero).
    pub weighted_length_q16: i64,
    /// Q16 length of the correlated component.
    pub correlated_length_q16: i64,
    /// Q16 length of the local component.
    pub local_length_q16: i64,
    /// The blend target magnitude: the stronger component length.
    pub target_magnitude_q16: i64,
    /// `weighted_length * Q16 / target_magnitude`, or 0 when the target is 0.
    pub weighted_over_target_q16: i64,
    /// `correlated.x * local.x + correlated.y * local.y` (Q32 scale).
    pub component_dot_q32: i64,
    /// True when the component dot product is negative.
    pub anti_aligned: bool,
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

/// Computes the pre-normalization weighted-blend intermediates.
///
/// This is the single source for both the produced displacement and the
/// near-zero diagnostics: [`blend_to_displacement_q16`] consumes the same
/// values this returns, so auditing this struct never diverges from the
/// generated geometry.
#[must_use]
pub fn weighted_blend_diagnostics(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
) -> WeightedBlendDiagnostics {
    let wc = i64::from(correlated_weight_q16);
    let wl = i64::from(local_weight_q16);
    let weighted_x = (correlated.x * wc + local.x * wl) / Q16;
    let weighted_y = (correlated.y * wc + local.y * wl) / Q16;
    let correlated_length = component_length_q16(correlated);
    let local_length = component_length_q16(local);
    let target = correlated_length.max(local_length);
    let weighted_length = if weighted_x == 0 && weighted_y == 0 {
        0
    } else {
        #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
        {
            let squared = (weighted_x * weighted_x + weighted_y * weighted_y) as u64;
            integer_sqrt(squared) as i64
        }
    };
    let weighted_over_target = if target == 0 {
        0
    } else {
        weighted_length * Q16 / target
    };
    let dot = correlated.x * local.x + correlated.y * local.y;
    WeightedBlendDiagnostics {
        weighted_x_q16: weighted_x,
        weighted_y_q16: weighted_y,
        weighted_length_q16: weighted_length,
        correlated_length_q16: correlated_length,
        local_length_q16: local_length,
        target_magnitude_q16: target,
        weighted_over_target_q16: weighted_over_target,
        component_dot_q32: dot,
        anti_aligned: dot < 0,
    }
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
    let diagnostics =
        weighted_blend_diagnostics(correlated, local, correlated_weight_q16, local_weight_q16);
    let (dir_x, dir_y) = if diagnostics.weighted_x_q16 == 0 && diagnostics.weighted_y_q16 == 0 {
        let length = diagnostics.local_length_q16;
        if length == 0 {
            (DIRECTION_SHIFT, 0)
        } else {
            (
                local.x * DIRECTION_SHIFT / length,
                local.y * DIRECTION_SHIFT / length,
            )
        }
    } else {
        let length = diagnostics.weighted_length_q16;
        (
            diagnostics.weighted_x_q16 * DIRECTION_SHIFT / length,
            diagnostics.weighted_y_q16 * DIRECTION_SHIFT / length,
        )
    };
    let magnitude = diagnostics.target_magnitude_q16;
    FixedVectorQ16 {
        x: dir_x * magnitude / DIRECTION_SHIFT,
        y: dir_y * magnitude / DIRECTION_SHIFT,
    }
}
