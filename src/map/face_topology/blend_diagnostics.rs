//! Shared fixed-point arithmetic behind the blend in [`super::blend`].
//!
//! Kept in its own module so both `blend.rs` and the reliability-floor
//! threshold scans can reach every intermediate without crossing the 300-line
//! per-file limit. None of this is Bevy state: it is pure deterministic math.

const Q16: i64 = 65_536;

use super::blend::{
    BlendReference, BlendStabilization, FixedVectorQ16, CORRELATED_PREFERENCE_MARGIN_Q16,
    MIN_RELIABLE_DIRECTION_RATIO_Q16,
};

/// Pre-normalization intermediates of one weighted blend.
///
/// The `weighted_*` fields are the raw pre-normalization values; the
/// `stabilized_*` fields replace them only below the reliability floor. All
/// arithmetic matches [`super::blend::blend_to_displacement_q16`] exactly, so
/// the diagnostics can never disagree with the displacement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WeightedBlendDiagnostics {
    pub weighted_x_q16: i64,
    pub weighted_y_q16: i64,
    pub weighted_length_q16: i64,
    pub correlated_length_q16: i64,
    pub local_length_q16: i64,
    pub target_magnitude_q16: i64,
    pub weighted_over_target_q16: i64,
    pub component_dot_q32: i64,
    pub anti_aligned: bool,
    pub reference: BlendReference,
    pub exact_zero: bool,
    pub stabilization_applied: bool,
    pub projection_q16: i64,
    pub correction_q16: i64,
    pub minimum_projection_q16: i64,
    pub stabilized_x_q16: i64,
    pub stabilized_y_q16: i64,
    pub stabilized_length_q16: i64,
    pub stabilized_ratio_q16: i64,
}

/// Deterministic floor integer square root (Newton, from above).
///
/// Normalization stays integer so the blend is exactly platform-independent.
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
#[must_use]
pub fn target_magnitude_q16(correlated: FixedVectorQ16, local: FixedVectorQ16) -> i64 {
    component_length_q16(correlated).max(component_length_q16(local))
}

/// The stabilization reference: the component with the larger weighted
/// magnitude, near-ties resolved toward the correlated (smooth, coherent)
/// component over the per-corner local noise.
#[must_use]
pub fn blend_reference(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
) -> BlendReference {
    let wc = i64::from(correlated_weight_q16);
    let wl = i64::from(local_weight_q16);
    let correlated_weighted_length = component_length_q16(correlated) * wc;
    let local_weighted_length = component_length_q16(local) * wl;
    if correlated_weighted_length == 0 && local_weighted_length == 0 {
        return BlendReference::FixedPositiveX;
    }
    if correlated_weighted_length == 0 {
        return BlendReference::Local;
    }
    if local_weighted_length == 0 {
        return BlendReference::Correlated;
    }
    let gap = (local_weighted_length - correlated_weighted_length).abs();
    let larger = local_weighted_length.max(correlated_weighted_length);
    if gap * Q16 < larger * CORRELATED_PREFERENCE_MARGIN_Q16 {
        return BlendReference::Correlated;
    }
    if correlated_weighted_length > local_weighted_length {
        BlendReference::Correlated
    } else {
        BlendReference::Local
    }
}

/// Applies the reliability-floor correction with a caller-supplied ratio (so
/// threshold scans don't disturb the constant the production blend uses).
#[must_use]
pub(crate) fn stabilize_blend_direction(
    weighted: FixedVectorQ16,
    weighted_length_q16: i64,
    target_magnitude_q16: i64,
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_length_q16: i64,
    local_length_q16: i64,
    reference: BlendReference,
    reliability_ratio_q16: i64,
) -> BlendStabilization {
    let minimum_projection = target_magnitude_q16 * reliability_ratio_q16 / Q16;
    let below_floor = target_magnitude_q16 > 0
        && weighted_length_q16 * Q16 / target_magnitude_q16 < reliability_ratio_q16;
    if !below_floor {
        return BlendStabilization {
            applied: false,
            projection_q16: 0,
            correction_q16: 0,
            minimum_projection_q16: minimum_projection,
            stabilized_x_q16: weighted.x,
            stabilized_y_q16: weighted.y,
            stabilized_length_q16: weighted_length_q16,
        };
    }
    let (ref_x, ref_y, ref_length) = match reference {
        BlendReference::Correlated => (correlated.x, correlated.y, correlated_length_q16),
        BlendReference::Local => (local.x, local.y, local_length_q16),
        BlendReference::FixedPositiveX => (1, 0, 1),
    };
    let projection = (weighted.x * ref_x + weighted.y * ref_y) / ref_length;
    let correction = minimum_projection - projection;
    let (stabilized_x, stabilized_y) = match reference {
        BlendReference::FixedPositiveX => (weighted.x + correction, weighted.y),
        BlendReference::Correlated | BlendReference::Local => (
            weighted.x + ref_x * correction / ref_length,
            weighted.y + ref_y * correction / ref_length,
        ),
    };
    let stabilized_length = if stabilized_x == 0 && stabilized_y == 0 {
        0
    } else {
        #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
        {
            let squared = (stabilized_x * stabilized_x + stabilized_y * stabilized_y) as u64;
            integer_sqrt(squared) as i64
        }
    };
    BlendStabilization {
        applied: true,
        projection_q16: projection,
        correction_q16: correction,
        minimum_projection_q16: minimum_projection,
        stabilized_x_q16: stabilized_x,
        stabilized_y_q16: stabilized_y,
        stabilized_length_q16: stabilized_length,
    }
}

/// Computes the pre-normalization weighted-blend intermediates; the single
/// source for both the produced displacement and the near-zero diagnostics.
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
    let reference = blend_reference(correlated, local, correlated_weight_q16, local_weight_q16);
    let stabilization = stabilize_blend_direction(
        FixedVectorQ16 {
            x: weighted_x,
            y: weighted_y,
        },
        weighted_length,
        target,
        correlated,
        local,
        correlated_length,
        local_length,
        reference,
        MIN_RELIABLE_DIRECTION_RATIO_Q16,
    );
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
        reference,
        exact_zero: target == 0,
        stabilization_applied: stabilization.applied,
        projection_q16: stabilization.projection_q16,
        correction_q16: stabilization.correction_q16,
        minimum_projection_q16: stabilization.minimum_projection_q16,
        stabilized_x_q16: stabilization.stabilized_x_q16,
        stabilized_y_q16: stabilization.stabilized_y_q16,
        stabilized_length_q16: stabilization.stabilized_length_q16,
        stabilized_ratio_q16: if target == 0 {
            0
        } else {
            stabilization.stabilized_length_q16 * Q16 / target
        },
    }
}
