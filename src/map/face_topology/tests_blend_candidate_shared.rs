//! Shared candidate-generation fixtures for the blend reliability-floor scans:
//! the seven candidate policies (raw, plus length/projection activation at
//! 1/64, 1/32, 1/16) and the helpers that build each candidate's own full
//! topology, so no candidate is ever measured by re-classifying another's
//! geometry.
#[cfg(test)]
pub(crate) mod shared {
    use crate::map::data::MapData;
    use crate::map::face_topology::blend::BlendReliabilityPolicy;
    use crate::map::face_topology::blend_diagnostics::weighted_blend_diagnostics_with_policy;
    use crate::map::face_topology::blend_policy::{
        candidate_policy, BlendActivation, DISABLED_BLEND_RELIABILITY_POLICY,
    };
    use crate::map::face_topology::generator::generate_hex_face_topology_with_profile_and_policy;
    use crate::map::face_topology::profiles::{
        interpolated_correlated_field, local_component_q16, HexDeformationProfile,
    };
    use crate::map::face_topology::types::{HexFaceTopology, SharedCornerKey};
    use crate::map::WorldSeed;
    use std::collections::HashSet;

    /// The profiles with a real blend (Subtle has no blend law at all).
    pub const BLENDED_PROFILES: [HexDeformationProfile; 2] = [
        HexDeformationProfile::Organic,
        HexDeformationProfile::PagoniaLike,
    ];

    /// One named candidate policy in the reliability-floor scan matrix.
    #[derive(Debug, Clone, Copy)]
    pub struct BlendCandidate {
        pub name: &'static str,
        pub policy: BlendReliabilityPolicy,
    }

    /// The seven candidates: raw legacy law, then the length/projection
    /// activation modes at 1/64 (production), 1/32, and 1/16 floors.
    pub fn candidates() -> Vec<BlendCandidate> {
        vec![
            BlendCandidate {
                name: "raw",
                policy: DISABLED_BLEND_RELIABILITY_POLICY,
            },
            BlendCandidate {
                name: "1/64_len",
                policy: candidate_policy(1_024, BlendActivation::WeightedLength),
            },
            BlendCandidate {
                name: "1/32_len",
                policy: candidate_policy(2_048, BlendActivation::WeightedLength),
            },
            BlendCandidate {
                name: "1/16_len",
                policy: candidate_policy(4_096, BlendActivation::WeightedLength),
            },
            BlendCandidate {
                name: "1/64_proj",
                policy: candidate_policy(1_024, BlendActivation::ReferenceProjection),
            },
            BlendCandidate {
                name: "1/32_proj",
                policy: candidate_policy(2_048, BlendActivation::ReferenceProjection),
            },
            BlendCandidate {
                name: "1/16_proj",
                policy: candidate_policy(4_096, BlendActivation::ReferenceProjection),
            },
        ]
    }

    /// Builds one candidate's own complete topology without panicking.
    pub fn try_generate(
        map: &MapData,
        seed: u32,
        profile: HexDeformationProfile,
        policy: BlendReliabilityPolicy,
    ) -> Result<HexFaceTopology, crate::map::face_topology::types::HexFaceTopologyError> {
        generate_hex_face_topology_with_profile_and_policy(
            map,
            WorldSeed::new(seed),
            profile,
            policy,
        )
    }

    /// Builds one candidate's own complete topology, panicking with context.
    pub fn generate(
        map: &MapData,
        seed: u32,
        profile: HexDeformationProfile,
        policy: BlendReliabilityPolicy,
    ) -> HexFaceTopology {
        try_generate(map, seed, profile, policy).unwrap_or_else(|error| {
            panic!("seed={seed} profile={profile:?} policy={policy:?}: {error:?}")
        })
    }

    /// The corners whose direction the candidate actually corrected.
    pub fn stabilized_keys(
        topology: &HexFaceTopology,
        seed: u32,
        profile: HexDeformationProfile,
        policy: BlendReliabilityPolicy,
    ) -> HashSet<SharedCornerKey> {
        let config = profile.config();
        topology
            .vertices
            .iter()
            .filter(|vertex| {
                weighted_blend_diagnostics_with_policy(
                    interpolated_correlated_field(seed, vertex.canonical_key, profile),
                    local_component_q16(seed, vertex.canonical_key, profile),
                    config.correlated_weight_q16,
                    config.local_weight_q16,
                    policy,
                )
                .stabilization_applied
            })
            .map(|vertex| vertex.canonical_key)
            .collect()
    }

    /// Unique edge key identified by canonical endpoints, ordered deterministically.
    pub type EdgeKey = (SharedCornerKey, SharedCornerKey);

    /// Tolerance threshold for near-antiparallel edge directions.
    pub const NEAR_ANTIPARALLEL_DOT_THRESHOLD: f32 = -0.9995;

    /// Extracts displacement-direction dot products for all unique logical edges.
    pub fn extract_edge_dots(
        topology: &HexFaceTopology,
    ) -> std::collections::HashMap<EdgeKey, f32> {
        let mut map = std::collections::HashMap::new();
        for (index, edge) in topology.half_edges.iter().enumerate() {
            if !edge.twin.is_none_or(|twin| index < twin.index()) {
                continue;
            }
            let origin = &topology.vertices[edge.origin.index()];
            let destination = &topology.vertices[edge.destination.index()];
            let (Ok(o_reg), Ok(d_reg)) = (
                crate::map::face_topology::corner_key::regular_corner_position(
                    origin.canonical_key,
                ),
                crate::map::face_topology::corner_key::regular_corner_position(
                    destination.canonical_key,
                ),
            ) else {
                continue;
            };
            let dot = (origin.position - o_reg)
                .normalize_or_zero()
                .dot((destination.position - d_reg).normalize_or_zero());
            let k1 = origin.canonical_key;
            let k2 = destination.canonical_key;
            map.insert((k1.min(k2), k1.max(k2)), dot);
        }
        map
    }

    /// Compares candidate edge directions against raw baseline edge directions.
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    pub fn compare_adjacency(
        topology: &HexFaceTopology,
        stabilized: &HashSet<SharedCornerKey>,
        raw_edges: &std::collections::HashMap<EdgeKey, f32>,
        seed: u32,
        min_dot_neither: &mut f32,
        min_dot_origin: &mut f32,
        min_dot_dest: &mut f32,
        min_dot_both: &mut f32,
        largest_imp: &mut (f32, u32, SharedCornerKey, SharedCornerKey, f32, f32),
        largest_reg: &mut (f32, u32, SharedCornerKey, SharedCornerKey, f32, f32),
        improved_edges: &mut usize,
        unchanged_edges: &mut usize,
        regressed_edges: &mut usize,
        newly_near_anti: &mut usize,
        removed_near_anti: &mut usize,
        newly_near_anti_stab: &mut usize,
        newly_exact_m1: &mut usize,
        removed_exact_m1: &mut usize,
        newly_exact_m1_stab: &mut usize,
    ) {
        let exact_m1_bits = (-1.0_f32).to_bits();
        for (index, edge) in topology.half_edges.iter().enumerate() {
            if !edge.twin.is_none_or(|twin| index < twin.index()) {
                continue;
            }
            let origin = &topology.vertices[edge.origin.index()];
            let destination = &topology.vertices[edge.destination.index()];
            let (Ok(o_reg), Ok(d_reg)) = (
                crate::map::face_topology::corner_key::regular_corner_position(
                    origin.canonical_key,
                ),
                crate::map::face_topology::corner_key::regular_corner_position(
                    destination.canonical_key,
                ),
            ) else {
                continue;
            };
            let dot_after = (origin.position - o_reg)
                .normalize_or_zero()
                .dot((destination.position - d_reg).normalize_or_zero());
            let k1 = origin.canonical_key;
            let k2 = destination.canonical_key;
            let edge_key = (k1.min(k2), k1.max(k2));

            let orig_stab = stabilized.contains(&k1);
            let dest_stab = stabilized.contains(&k2);

            if !orig_stab && !dest_stab {
                *min_dot_neither = min_dot_neither.min(dot_after);
            } else if orig_stab && !dest_stab {
                *min_dot_origin = min_dot_origin.min(dot_after);
            } else if !orig_stab && dest_stab {
                *min_dot_dest = min_dot_dest.min(dot_after);
            } else {
                *min_dot_both = min_dot_both.min(dot_after);
            }

            if let Some(&dot_before) = raw_edges.get(&edge_key) {
                let delta = dot_after - dot_before;
                if delta > largest_imp.0 {
                    *largest_imp = (delta, seed, k1, k2, dot_before, dot_after);
                }
                if delta < largest_reg.0 {
                    *largest_reg = (delta, seed, k1, k2, dot_before, dot_after);
                }

                if dot_after > dot_before {
                    *improved_edges += 1;
                } else if dot_after < dot_before {
                    *regressed_edges += 1;
                } else {
                    *unchanged_edges += 1;
                }

                let before_near_anti = dot_before <= NEAR_ANTIPARALLEL_DOT_THRESHOLD;
                let after_near_anti = dot_after <= NEAR_ANTIPARALLEL_DOT_THRESHOLD;
                if !before_near_anti && after_near_anti {
                    *newly_near_anti += 1;
                    if orig_stab || dest_stab {
                        *newly_near_anti_stab += 1;
                    }
                }
                if before_near_anti && !after_near_anti {
                    *removed_near_anti += 1;
                }

                let before_exact_m1 = dot_before.to_bits() == exact_m1_bits;
                let after_exact_m1 = dot_after.to_bits() == exact_m1_bits;
                if !before_exact_m1 && after_exact_m1 {
                    *newly_exact_m1 += 1;
                    if orig_stab || dest_stab {
                        *newly_exact_m1_stab += 1;
                    }
                }
                if before_exact_m1 && !after_exact_m1 {
                    *removed_exact_m1 += 1;
                }
            }
        }
    }
}
