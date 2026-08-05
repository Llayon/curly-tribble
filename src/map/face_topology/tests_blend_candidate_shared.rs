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

    /// Builds one candidate's own complete topology, panicking with context.
    pub fn generate(
        map: &MapData,
        seed: u32,
        profile: HexDeformationProfile,
        policy: BlendReliabilityPolicy,
    ) -> HexFaceTopology {
        generate_hex_face_topology_with_profile_and_policy(
            map,
            WorldSeed::new(seed),
            profile,
            policy,
        )
        .unwrap_or_else(|error| {
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
}
