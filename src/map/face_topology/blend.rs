//! Deterministic fixed-point blend of deformation components.
//!
//! [`blend.rs`]: weights decide the *direction* (Q24 normalized) while the
//! magnitude comes from the stronger component. When the weighted sum nearly
//! cancels, its direction is dominated by rounding noise and flips between
//! adjacent corners; such *unreliable* corners (weighted length below
//! [`MIN_RELIABLE_DIRECTION_RATIO_Q16`]) are projected onto a continuous
//! *reference* (the stronger component, near-ties to the smooth correlated
//! field via [`CORRELATED_PREFERENCE_MARGIN_Q16`]). Reliable corners keep the
//! exact previous arithmetic, bit for bit. The shared arithmetic lives in
//! [`blend_diagnostics`].

/// Weighted-length ratio below which a blend direction is unreliable
/// (Q16 units: `1_024` == `1/64`).
pub const MIN_RELIABLE_DIRECTION_RATIO_Q16: i64 = 1_024;

/// Reference tie-break band as a ratio of the larger weighted length
/// (Q16 units: `8_192` == `1/8`).
///
/// A corner below the ratio floor has weighted sum `<= ratio * target` while
/// its larger weighted component is `>= min(wc, wl) * target`; the resulting
/// gap (6.25% here) routes every stabilized corner to the coherent correlated
/// field instead of per-corner local noise.
pub const CORRELATED_PREFERENCE_MARGIN_Q16: i64 = 8_192;

/// A 2D fixed-point vector (1.0 == `65_536`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedVectorQ16 {
    pub x: i64,
    pub y: i64,
}

/// The component a stabilized blend direction is projected onto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendReference {
    /// The correlated component (near-ties prefer this smooth field).
    Correlated,
    /// The local component.
    Local,
    /// No component has mass (both are exactly zero); use +X.
    FixedPositiveX,
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
}

pub use super::blend_diagnostics::{
    blend_reference, component_length_q16, integer_sqrt, target_magnitude_q16,
    weighted_blend_diagnostics, WeightedBlendDiagnostics,
};

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
    const DIRECTION_SHIFT: i64 = 1 << 24;
    let diagnostics =
        weighted_blend_diagnostics(correlated, local, correlated_weight_q16, local_weight_q16);
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
