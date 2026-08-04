//! Shared fixtures and measured-quality helpers for the hex face topology
//! quality regression suite: canonical map, stress shapes, fast seeds, and
//! per-case measured quality, so individual test files stay non-duplicative.
#[cfg(test)]
pub(crate) mod shared {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::acceptance::{
        ProfileAcceptanceCriteria, ProfileAcceptanceReport, DISPLACEMENT_CAP_EPSILON,
    };
    use crate::map::face_topology::fingerprint::topology_fingerprints;
    use crate::map::face_topology::generator::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::types::{HalfEdgeId, HexFaceTopology};
    use crate::map::face_topology::validate_complete_topology;
    use crate::map::{HexCoord, WorldSeed};
    use std::collections::HashSet;

    /// The eight deterministic seeds used by the fast stress tiers.
    pub const FAST_SEEDS: [u32; 8] = [0, 1, 7, 42, 99, 128, 200, 255];
    /// Number of hex faces on the canonical 40x40 map.
    pub const CANONICAL_FACES: usize = 1_600;
    /// Number of canonical corner vertices on the 40x40 map.
    pub const CANONICAL_VERTICES: usize = 3_360;
    /// Number of directed half-edges on the 40x40 map.
    pub const CANONICAL_HALF_EDGES: usize = 9_600;
    /// Number of paired (internal) undirected edges on the 40x40 map.
    pub const CANONICAL_PAIRED_EDGES: usize = 4_641;
    /// Number of border (unpaired) edges on the 40x40 map.
    pub const CANONICAL_BORDER_EDGES: usize = 318;
    /// Unique logical edges = paired + border on the 40x40 map.
    pub const CANONICAL_UNIQUE_LOGICAL_EDGES: usize = 4_959;

    pub fn tile() -> TileData {
        TileData::default()
    }

    /// The canonical 40x40 studio map used by every separation and fingerprint
    /// contract.
    pub fn map_40x40() -> MapData {
        let mut map = MapData::default();
        for r in 0..40 {
            let offset = r >> 1;
            for q in -offset..(40 - offset) {
                map.tiles.insert(HexCoord::new(q, r), tile());
            }
        }
        map.width = 40;
        map.height = 40;
        map
    }

    pub fn isolated_hex() -> MapData {
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), tile());
        map
    }

    pub fn two_neighbors() -> MapData {
        let mut map = isolated_hex();
        map.tiles.insert(HexCoord::new(1, 0), tile());
        map
    }

    pub fn seven_hex_cluster() -> MapData {
        let mut map = isolated_hex();
        for neighbor in HexCoord::new(0, 0).neighbors() {
            map.tiles.insert(neighbor, tile());
        }
        map
    }

    pub fn sparse_l_shape() -> MapData {
        let mut map = MapData::default();
        for coord in [
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(2, 0),
            HexCoord::new(0, 1),
            HexCoord::new(0, 2),
        ] {
            map.tiles.insert(coord, tile());
        }
        map
    }

    pub fn diagonal_strip() -> MapData {
        let mut map = MapData::default();
        for coord in [
            HexCoord::new(0, 0),
            HexCoord::new(1, 1),
            HexCoord::new(2, 2),
            HexCoord::new(3, 3),
        ] {
            map.tiles.insert(coord, tile());
        }
        map
    }

    /// The six stress shapes: one canonical and five small/irregular layouts.
    pub fn all_shapes() -> Vec<(&'static str, MapData)> {
        vec![
            ("40x40", map_40x40()),
            ("isolated", isolated_hex()),
            ("two_neighbors", two_neighbors()),
            ("seven_hex", seven_hex_cluster()),
            ("l_shape", sparse_l_shape()),
            ("diagonal", diagonal_strip()),
        ]
    }

    pub fn all_profiles() -> [HexDeformationProfile; 3] {
        [
            HexDeformationProfile::Subtle,
            HexDeformationProfile::Organic,
            HexDeformationProfile::PagoniaLike,
        ]
    }

    /// Generates one topology, panicking with full context on failure.
    pub fn generate(map: &MapData, seed: u32, profile: HexDeformationProfile) -> HexFaceTopology {
        generate_hex_face_topology_with_profile(map, WorldSeed::new(seed), profile)
            .unwrap_or_else(|error| panic!("seed={seed} profile={profile:?}: {error:?}"))
    }

    /// Per-case measured visual quality, without any profile-average bands.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct CaseQuality {
        pub average_displacement_ratio: f32,
        pub maximum_displacement_ratio: f32,
        pub minimum_edge_length_ratio: f32,
        pub minimum_interior_angle_degrees: f32,
        pub maximum_interior_angle_degrees: f32,
        pub minimum_aspect_quality: f32,
    }

    pub fn measured_quality(
        map: &MapData,
        seed: u32,
        profile: HexDeformationProfile,
    ) -> CaseQuality {
        let report = ProfileAcceptanceReport::from_topology(&generate(map, seed, profile));
        CaseQuality {
            average_displacement_ratio: report.average_displacement_ratio,
            maximum_displacement_ratio: report.maximum_displacement_ratio,
            minimum_edge_length_ratio: report.minimum_edge_length_ratio,
            minimum_interior_angle_degrees: report.minimum_interior_angle_degrees,
            maximum_interior_angle_degrees: report.maximum_interior_angle_degrees,
            minimum_aspect_quality: report.minimum_aspect_quality,
        }
    }

    /// Runs the shared structural, cap, acceptance, and zero-backoff checks.
    ///
    /// Returns one human-readable failure string per violated contract, empty
    /// when the case fully passes. Used by both the fast matrix and the full
    /// 4,608 stress tier so the two never drift apart.
    #[allow(clippy::too_many_lines)]
    pub fn core_failures(
        map: &MapData,
        shape: &str,
        seed: u32,
        profile: HexDeformationProfile,
    ) -> Vec<String> {
        let context = format!("shape={shape} seed={seed} profile={profile:?}");
        let topology = generate(map, seed, profile);
        let mut failures = Vec::new();
        if let Err(error) = validate_complete_topology(&topology, map) {
            return vec![format!("{context}: complete validation failed: {error:?}")];
        }
        let report = ProfileAcceptanceReport::from_topology(&topology);
        if !report.has_finite_metrics() {
            failures.push(format!("{context}: non-finite metrics: {report:?}"));
        }
        let cap_ratio = profile.config().absolute_displacement_cap_ratio();
        if report.maximum_displacement_ratio > cap_ratio + DISPLACEMENT_CAP_EPSILON {
            failures.push(format!(
                "{context}: max displacement ratio {} exceeds cap {}",
                report.maximum_displacement_ratio, cap_ratio
            ));
        }
        let issues = report.violations(ProfileAcceptanceCriteria::for_profile(profile));
        if !issues.is_empty() {
            failures.push(format!("{context}: acceptance issues: {issues:?}"));
        }
        if topology.faces.len() != map.tiles.len() {
            failures.push(format!(
                "{context}: face count {} != tile count {}",
                topology.faces.len(),
                map.tiles.len()
            ));
        }
        for (face_index, face) in topology.faces.iter().enumerate() {
            let mut visited = HashSet::new();
            for vertex_id in face.vertices {
                if !visited.insert(vertex_id) {
                    failures.push(format!(
                        "{context}: face {face_index} repeats vertex {vertex_id:?}"
                    ));
                    break;
                }
            }
            let mut edge = face.boundary;
            for step in 0..6 {
                let current = &topology.half_edges[edge.index()];
                if current.incident_face.index() != face_index {
                    failures.push(format!(
                        "{context}: face {face_index} edge {step} ownership mismatch"
                    ));
                }
                edge = current.next;
            }
            if edge != face.boundary {
                failures.push(format!(
                    "{context}: face {face_index} boundary does not close"
                ));
            }
        }
        for (edge_index, edge) in topology.half_edges.iter().enumerate() {
            if let Some(twin_id) = edge.twin {
                let twin = &topology.half_edges[twin_id.index()];
                if twin.twin != Some(HalfEdgeId::new(edge_index))
                    || twin.origin != edge.destination
                    || twin.destination != edge.origin
                {
                    failures.push(format!("{context}: edge {edge_index} twin not symmetric"));
                }
            }
        }
        assert_zero_backoff(&topology, &context, &mut failures);
        if topology.half_edges.len() != topology.stats.half_edge_count {
            failures.push(format!(
                "{context}: half_edge_count {} != stored {}",
                topology.half_edges.len(),
                topology.stats.half_edge_count
            ));
        }
        if topology.stats.paired_edge_count * 2 + topology.stats.border_edge_count
            != topology.half_edges.len()
        {
            failures.push(format!(
                "{context}: pair/border arithmetic inconsistent with half-edges: {}",
                topology.stats.border_edge_count
            ));
        }
        failures
    }

    /// The fast-tier case checks: core checks plus repeat determinism and
    /// fingerprint stability.
    pub fn case_failures(
        map: &MapData,
        shape: &str,
        seed: u32,
        profile: HexDeformationProfile,
    ) -> Vec<String> {
        let mut failures = core_failures(map, shape, seed, profile);
        let first = generate(map, seed, profile);
        let second = generate(map, seed, profile);
        if first != second {
            failures.push(format!(
                "shape={shape} seed={seed} profile={profile:?}: generation is not deterministic"
            ));
        }
        let first_fp = topology_fingerprints(map, WorldSeed::new(seed), &first);
        let second_fp = topology_fingerprints(map, WorldSeed::new(seed), &second);
        if first_fp != second_fp {
            failures.push(format!(
                "shape={shape} seed={seed} profile={profile:?}: fingerprints differ across repeats: {first_fp:?} {second_fp:?}"
            ));
        }
        failures
    }

    fn assert_zero_backoff(topology: &HexFaceTopology, context: &str, failures: &mut Vec<String>) {
        if topology.stats.reduction_rounds != 0 {
            failures.push(format!(
                "{context}: reduction_rounds={} (expected zero)",
                topology.stats.reduction_rounds
            ));
        }
        if topology.stats.reduced_vertices != 0 {
            failures.push(format!(
                "{context}: reduced_vertices={} (expected zero)",
                topology.stats.reduced_vertices
            ));
        }
        if topology.stats.reduced_displacement_fallbacks != 0 {
            failures.push(format!(
                "{context}: reduced_displacement_fallbacks={} (expected zero)",
                topology.stats.reduced_displacement_fallbacks
            ));
        }
        if topology.stats.regular_position_fallbacks != 0 {
            failures.push(format!(
                "{context}: regular_position_fallbacks={} (expected zero)",
                topology.stats.regular_position_fallbacks
            ));
        }
    }
}
