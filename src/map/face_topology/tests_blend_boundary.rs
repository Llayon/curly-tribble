//! Permanent boundary assertions for the reliability floor.
//!
//! The floor is an *open* upper bound: a corner whose weighted length exactly
//! equals the floor ratio keeps the raw law, and only `floor - 1` and below
//! are corrected. This is guaranteed exactly because the predicate is
//! cross-multiplied in `i128` (no intermediate division can round a boundary
//! case). `floor + 1` and `floor + 2` are well above and necessarily raw.
#[cfg(test)]
mod blend_boundary_tests {
    use crate::map::face_topology::blend::{
        blend_to_displacement_q16_with_policy, BlendReliabilityPolicy, FixedVectorQ16,
    };
    use crate::map::face_topology::blend_diagnostics::weighted_blend_diagnostics_with_policy;
    use crate::map::face_topology::blend_policy::BlendActivation;

    const Q16: i64 = 65_536;
    const FLOOR: i64 = 1_024;

    fn vector(x: i64, y: i64) -> FixedVectorQ16 {
        FixedVectorQ16 { x, y }
    }

    fn policy(ratio: i64, activation: BlendActivation) -> BlendReliabilityPolicy {
        BlendReliabilityPolicy::new(ratio, 8_192, activation)
    }

    /// A positively-aligned synthetic corner: weighted length `weight` (Q16
    /// units) against a target of one full `Q16`, so `ratio == weight` exactly.
    fn probe(weight: i64, activation: BlendActivation) -> (bool, FixedVectorQ16) {
        let correlated = vector(Q16, 0);
        let local = vector(0, 0);
        let diagnostics = weighted_blend_diagnostics_with_policy(
            correlated,
            local,
            u32::try_from(weight).unwrap(),
            u32::try_from(weight).unwrap(),
            policy(FLOOR, activation),
        );
        let produced = blend_to_displacement_q16_with_policy(
            correlated,
            local,
            u32::try_from(weight).unwrap(),
            u32::try_from(weight).unwrap(),
            policy(FLOOR, activation),
        );
        (diagnostics.stabilization_applied, produced)
    }

    /// The boundary is an open upper bound for the length activation: exactly
    /// the floor (and floor+1, floor+2) keep the raw law; only floor-1 and
    /// floor-2 are corrected. The corrected and raw outputs agree on a +X
    /// parallel input, so the law is asserted on the correction trigger.
    #[test]
    fn weighted_length_floor_boundary_is_an_open_upper_bound() {
        let (below_applied, below_output) = probe(FLOOR - 1, BlendActivation::WeightedLength);
        assert!(
            below_applied,
            "floor-1 is below the floor and must stabilize"
        );
        let (floor_applied, floor_output) = probe(FLOOR, BlendActivation::WeightedLength);
        assert!(!floor_applied, "the floor itself is never below the floor");
        for weight in [FLOOR + 1, FLOOR + 2] {
            let (applied, _) = probe(weight, BlendActivation::WeightedLength);
            assert!(
                !applied,
                "weighted {weight} is above the floor and stays raw"
            );
        }
        let _ = probe(FLOOR - 2, BlendActivation::WeightedLength);
        assert_eq!(
            below_output, floor_output,
            "raw and corrected agree on a parallel input"
        );
    }

    /// The same open-boundary law under the projection activation mode, where
    /// the measured quantity is the projection of the weighted sum onto the
    /// reference (identical to the length here, since the weighted sum is
    /// already along +X).
    #[test]
    fn projection_floor_boundary_is_an_open_upper_bound() {
        let (below_applied, below_output) = probe(FLOOR - 1, BlendActivation::ReferenceProjection);
        assert!(below_applied, "projection floor-1 must stabilize");
        let (floor_applied, floor_output) = probe(FLOOR, BlendActivation::ReferenceProjection);
        assert!(!floor_applied, "projection floor stays raw");
        for weight in [FLOOR + 1, FLOOR + 2] {
            let (applied, _) = probe(weight, BlendActivation::ReferenceProjection);
            assert!(!applied, "projection weight {weight} stays raw");
        }
        let _ = probe(FLOOR - 2, BlendActivation::ReferenceProjection);
        assert_eq!(below_output, floor_output);
    }

    /// The cross-multiplied predicate is exact at the boundary: no division in
    /// the comparison, a zero target never triggers, and a negative projection
    /// (below any non-negative floor) always triggers.
    #[test]
    fn is_below_floor_is_exact_at_the_boundary_and_for_edge_targets() {
        for activation in [
            BlendActivation::WeightedLength,
            BlendActivation::ReferenceProjection,
        ] {
            let law = policy(FLOOR, activation);
            assert!(
                !law.is_below_floor(FLOOR, FLOOR, Q16),
                "floor is not below the floor"
            );
            assert!(
                law.is_below_floor(FLOOR - 1, FLOOR - 1, Q16),
                "floor-1 is below"
            );
            assert!(
                law.is_below_floor(FLOOR - 2, FLOOR - 2, Q16),
                "floor-2 is below"
            );
            assert!(
                !law.is_below_floor(FLOOR + 1, FLOOR + 1, Q16),
                "floor+1 is above"
            );
            assert!(
                !law.is_below_floor(FLOOR + 2, FLOOR + 2, Q16),
                "floor+2 is above"
            );
            assert!(!law.is_below_floor(0, 0, 0), "zero target never triggers");
            assert!(
                !law.is_below_floor(0, 0, -1),
                "negative target never triggers"
            );
            assert!(
                law.is_below_floor(0, -5, Q16),
                "a negative projection is always below a non-negative floor"
            );
        }
    }
}
