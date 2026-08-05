//! Deterministic fixed-point blend of deformation components.
//!
//! [`blend.rs`]: weights decide the *direction* (Q24 normalized) while the
//! magnitude comes from the stronger component. When the weighted sum nearly
//! cancels, its direction is dominated by rounding noise and flips between
//! adjacent corners; such *unreliable* corners (weighted length below the
//! policy floor) are projected onto a continuous *reference* (the stronger
//! component, near-ties to the smooth correlated field via the correlated
//! preference margin). Reliable corners keep the exact previous arithmetic,
//! bit for bit. The shared arithmetic lives in [`blend_diagnostics`]; the law
//! (thresholds, activation mode, margin) lives in [`blend_policy`].

/// A 2D fixed-point vector (1.0 == `65_536`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedVectorQ16 {
    pub x: i64,
    pub y: i64,
}

/// Outcome of the reliability-floor correction (all Q16 fixed-point).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BlendStabilization {
    pub(crate) applied: bool,
    pub(crate) projection_q16: i64,
    pub(crate) correction_q16: i64,
    pub(crate) minimum_projection_q16: i64,
    pub(crate) stabilized_x_q16: i64,
    pub(crate) stabilized_y_q16: i64,
    pub(crate) stabilized_length_q16: i64,
    pub(crate) stabilized_projection_q16: i64,
}

pub use super::blend_diagnostics::{
    component_length_q16, integer_sqrt, projection_onto_reference_q16, resolve_reference,
    target_magnitude_q16, weighted_blend_diagnostics, weighted_blend_diagnostics_with_policy,
    BlendReference, ResolvedBlendReference, WeightedBlendDiagnostics,
};
pub use super::blend_policy::{
    BlendActivation, BlendReliabilityPolicy, CORRELATED_PREFERENCE_MARGIN_Q16,
    MIN_RELIABLE_DIRECTION_RATIO_Q16, PRODUCTION_BLEND_RELIABILITY_POLICY,
};

const Q16: i64 = 65_536;

/// The stabilization reference for the default production margin.
#[must_use]
pub fn blend_reference(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
) -> BlendReference {
    blend_reference_with_margin(
        correlated,
        local,
        correlated_weight_q16,
        local_weight_q16,
        CORRELATED_PREFERENCE_MARGIN_Q16,
    )
}

/// The stabilization reference: the component with the larger weighted
/// magnitude, near-ties resolved toward the correlated (smooth, coherent)
/// component over the per-corner local noise.
#[must_use]
pub(crate) fn blend_reference_with_margin(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
    correlated_preference_margin_q16: i64,
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
    if gap * Q16 < larger * correlated_preference_margin_q16 {
        return BlendReference::Correlated;
    }
    if correlated_weighted_length > local_weighted_length {
        BlendReference::Correlated
    } else {
        BlendReference::Local
    }
}

/// Blends two components into a Q16 displacement vector.
///
/// Direction is the profile-weighted sum normalized in Q24, replaced below the
/// reliability floor by the projection-floor toward the reference (see module
/// docs); an exactly-zero sum with no correction falls back to the local
/// direction. Magnitude is [`target_magnitude_q16`]. All arithmetic is integer.
#[must_use]
pub fn blend_to_displacement_q16(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
) -> FixedVectorQ16 {
    blend_to_displacement_q16_with_policy(
        correlated,
        local,
        correlated_weight_q16,
        local_weight_q16,
        PRODUCTION_BLEND_RELIABILITY_POLICY,
    )
}

/// Like [`blend_to_displacement_q16`], but under an explicit policy, so the
/// candidate generator can build genuinely different geometry for each
/// threshold instead of re-classifying a single topology.
#[must_use]
pub fn blend_to_displacement_q16_with_policy(
    correlated: FixedVectorQ16,
    local: FixedVectorQ16,
    correlated_weight_q16: u32,
    local_weight_q16: u32,
    policy: BlendReliabilityPolicy,
) -> FixedVectorQ16 {
    const DIRECTION_SHIFT: i64 = 1 << 24;
    let diagnostics = weighted_blend_diagnostics_with_policy(
        correlated,
        local,
        correlated_weight_q16,
        local_weight_q16,
        policy,
    );
    let (dir_x, dir_y) = if diagnostics.stabilization_applied {
        let length = diagnostics.stabilized_length_q16;
        if length == 0 {
            (DIRECTION_SHIFT, 0)
        } else {
            (
                diagnostics.stabilized_x_q16 * DIRECTION_SHIFT / length,
                diagnostics.stabilized_y_q16 * DIRECTION_SHIFT / length,
            )
        }
    } else if diagnostics.weighted_x_q16 == 0 && diagnostics.weighted_y_q16 == 0 {
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
