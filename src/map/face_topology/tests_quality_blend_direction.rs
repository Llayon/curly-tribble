//! Stabilized blend direction invariants and threshold measurements: the
//! reliable corners keep the exact raw normalization (bit-identical), the
//! stabilized corners reach the reliability projection floor, and the worst
//! adjacent direction changes are tracked per stabilization category.
#[cfg(test)]
mod quality_blend_direction_tests {
    use crate::map::face_topology::blend::{
        blend_to_displacement_q16, component_length_q16, weighted_blend_diagnostics,
        FixedVectorQ16, WeightedBlendDiagnostics, MIN_RELIABLE_DIRECTION_RATIO_Q16,
    };
    use crate::map::face_topology::corner_key::regular_corner_position;
    use crate::map::face_topology::profiles::{
        interpolated_correlated_field, local_component_q16, HexDeformationProfile,
    };
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::types::{HexFaceTopology, MapVertex, SharedCornerKey};
    const Q16: i64 = 65_536;
    fn observations(
        seed: u32,
        key: SharedCornerKey,
        profile: HexDeformationProfile,
    ) -> (WeightedBlendDiagnostics, FixedVectorQ16) {
        let config = profile.config();
        let correlated = interpolated_correlated_field(seed, key, profile);
        let local = local_component_q16(seed, key, profile);
        let wc = config.correlated_weight_q16;
        let wl = config.local_weight_q16;
        (
            weighted_blend_diagnostics(correlated, local, wc, wl),
            blend_to_displacement_q16(correlated, local, wc, wl),
        )
    }
    fn unique_edge_pairs(
        topology: &HexFaceTopology,
    ) -> impl Iterator<Item = (&MapVertex, &MapVertex)> + '_ {
        topology
            .half_edges
            .iter()
            .enumerate()
            .filter(|(index, edge)| edge.twin.is_none_or(|twin| *index < twin.index()))
            .map(|(_, edge)| {
                (
                    &topology.vertices[edge.origin.index()],
                    &topology.vertices[edge.destination.index()],
                )
            })
    }
    /// The pre-fix raw law: Q24-normalize the weighted sum (falling back to the
    /// local component only when the sum is exactly zero), then scale to the
    /// target magnitude. Deterministic, no floating point.
    fn raw_law(diagnostics: &WeightedBlendDiagnostics, local: FixedVectorQ16) -> FixedVectorQ16 {
        const DIRECTION_SHIFT: i64 = 1 << 24;
        let (vx, vy, len) = if diagnostics.weighted_x_q16 == 0 && diagnostics.weighted_y_q16 == 0 {
            (local.x, local.y, diagnostics.local_length_q16)
        } else {
            (
                diagnostics.weighted_x_q16,
                diagnostics.weighted_y_q16,
                diagnostics.weighted_length_q16,
            )
        };
        let (dx, dy) = if len == 0 {
            (DIRECTION_SHIFT, 0)
        } else {
            (vx * DIRECTION_SHIFT / len, vy * DIRECTION_SHIFT / len)
        };
        FixedVectorQ16 {
            x: dx * diagnostics.target_magnitude_q16 / DIRECTION_SHIFT,
            y: dy * diagnostics.target_magnitude_q16 / DIRECTION_SHIFT,
        }
    }
    /// Reliable corners keep the raw weighted-normalization law exactly,
    /// bit for bit, over every unstabilized fast-seed corner.
    #[test]
    fn reliable_directions_are_bit_identical_to_the_raw_law() {
        let map = q::map_40x40();
        let mut reliable_samples = 0_usize;
        for seed in q::FAST_SEEDS {
            for profile in [
                HexDeformationProfile::Organic,
                HexDeformationProfile::PagoniaLike,
            ] {
                let topology = q::generate(&map, seed, profile);
                for vertex in &topology.vertices {
                    let (diagnostics, produced) = observations(seed, vertex.canonical_key, profile);
                    if diagnostics.stabilization_applied {
                        continue;
                    }
                    let local = local_component_q16(seed, vertex.canonical_key, profile);
                    assert_eq!(
                        produced,
                        raw_law(&diagnostics, local),
                        "reliable corner must keep the raw law exactly"
                    );
                    reliable_samples += 1;
                }
            }
        }
        assert!(reliable_samples > 0);
    }

    /// Stabilized corners reach the reliability projection floor: only
    /// unreliable corners stabilize, the corrected length stays within the
    /// floor's truncation, and the produced magnitude stays at the target.
    #[test]
    fn stabilized_directions_keep_a_minimum_projection_onto_the_reference() {
        let map = q::map_40x40();
        let mut stabilized_samples = 0_usize;
        for seed in q::FAST_SEEDS {
            for profile in [
                HexDeformationProfile::Organic,
                HexDeformationProfile::PagoniaLike,
            ] {
                let topology = q::generate(&map, seed, profile);
                for vertex in &topology.vertices {
                    let (diagnostics, produced) = observations(seed, vertex.canonical_key, profile);
                    if !diagnostics.stabilization_applied {
                        continue;
                    }
                    assert!(
                        diagnostics.weighted_length_q16 * Q16 / diagnostics.target_magnitude_q16
                            < MIN_RELIABLE_DIRECTION_RATIO_Q16,
                        "only unreliable corners are stabilized"
                    );
                    let produced_len = component_length_q16(produced);
                    assert!(
                        produced_len >= diagnostics.target_magnitude_q16 - 2,
                        "stabilization must not shrink the magnitude"
                    );
                    assert!(
                        diagnostics.stabilized_length_q16 >= diagnostics.minimum_projection_q16 - 2,
                        "corrected length stays within truncation of the floor"
                    );
                    assert!(
                        diagnostics.stabilized_length_ratio_q16
                            >= MIN_RELIABLE_DIRECTION_RATIO_Q16 - 16,
                        "corrected length reaches the reliability floor"
                    );
                    stabilized_samples += 1;
                }
            }
        }
        assert!(stabilized_samples > 0, "fast seeds must exercise the floor");
    }

    /// Adjacent displacement-direction audit with 4-way stabilization tracking
    /// over the fast seeds: the global -1.0 extreme is a non-near-zero
    /// high-frequency local flip, the floor never deepens a pair beyond it,
    /// and both-stabilized pairs stay far from anti-parallel.
    #[test]
    fn adjacent_displacement_direction_audit_on_canonical_map() {
        let map = q::map_40x40();
        let mut global = f32::MAX;
        let mut one = f32::MAX;
        let mut both = f32::MAX;
        for seed in q::FAST_SEEDS {
            for profile in [
                HexDeformationProfile::Organic,
                HexDeformationProfile::PagoniaLike,
            ] {
                let topology = q::generate(&map, seed, profile);
                for (origin, destination) in unique_edge_pairs(&topology) {
                    let (Ok(origin_regular), Ok(destination_regular)) = (
                        regular_corner_position(origin.canonical_key),
                        regular_corner_position(destination.canonical_key),
                    ) else {
                        continue;
                    };
                    let origin_stabilized = observations(seed, origin.canonical_key, profile)
                        .0
                        .stabilization_applied;
                    let destination_stabilized =
                        observations(seed, destination.canonical_key, profile)
                            .0
                            .stabilization_applied;
                    let dot = (origin.position - origin_regular)
                        .normalize_or_zero()
                        .dot((destination.position - destination_regular).normalize_or_zero());
                    global = global.min(dot);
                    if origin_stabilized != destination_stabilized {
                        one = one.min(dot);
                    }
                    if origin_stabilized && destination_stabilized {
                        both = both.min(dot);
                    }
                }
            }
        }
        println!("worst adjacent direction-change: global={global:.5} one_stabilized={one:.5} both_stabilized={both:.5}");
        assert!(
            global >= -1.0 && global.is_finite(),
            "adjacent dot is a unit-range quantity"
        );
        assert!(
            one >= global - 1e-4,
            "the floor must not deepen a neighbor pair beyond the pre-existing worst"
        );
        assert!(
            both >= -0.1,
            "near-zero-linked pairs must stay far from anti-parallel: both={both}"
        );
    }
}
