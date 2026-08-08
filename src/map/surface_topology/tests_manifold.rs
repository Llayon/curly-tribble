// src/map/surface_topology/tests_manifold.rs
//! Global manifold topology proof.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTopologyManifoldTestsPlugin;

impl Plugin for SurfaceTopologyManifoldTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::surface_topology::validation::validate_surface_topology;
    use crate::map::HexCoord;

    #[test]
    fn global_manifold_proof_on_cluster_map() {
        let mut map = MapData::default();
        for q in 0..3 {
            for r in 0..3 {
                map.tiles.insert(HexCoord::new(q, r), TileData::default());
            }
        }

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::PagoniaLike)
                .expect("Face topology failed");

        let surface = generate_surface_topology(&face_topology).expect("Surface topology failed");

        // Validate surface topology invariants
        validate_surface_topology(&surface).expect("Validation failed");

        assert_eq!(surface.faces.len(), 9 * 24);
        assert_eq!(surface.half_edges.len(), 9 * 24 * 3);

        // Independent manifold audit over half-edges
        let mut edge_buckets: std::collections::HashMap<(usize, usize), Vec<usize>> =
            std::collections::HashMap::new();

        for (idx, he) in surface.half_edges.iter().enumerate() {
            let u0 = he.origin.index();
            let u1 = he.destination.index();
            let key = (u0.min(u1), u0.max(u1));
            edge_buckets.entry(key).or_default().push(idx);
        }

        let mut boundary_count = 0;
        let mut paired_count = 0;

        for (&(u0, u1), bucket) in &edge_buckets {
            match bucket.len() {
                1 => {
                    boundary_count += 1;
                    let he = &surface.half_edges[bucket[0]];
                    assert!(
                        he.twin.is_none(),
                        "Boundary half-edge must have twin == None"
                    );
                }
                2 => {
                    paired_count += 2;
                    let h_a_idx = bucket[0];
                    let h_b_idx = bucket[1];
                    let h_a = &surface.half_edges[h_a_idx];
                    let h_b = &surface.half_edges[h_b_idx];

                    assert_eq!(
                        h_a.twin,
                        Some(crate::map::surface_topology::types::SurfaceHalfEdgeId::new(
                            h_b_idx
                        ))
                    );
                    assert_eq!(
                        h_b.twin,
                        Some(crate::map::surface_topology::types::SurfaceHalfEdgeId::new(
                            h_a_idx
                        ))
                    );
                    assert_eq!(h_a.origin.index(), h_b.destination.index());
                    assert_eq!(h_a.destination.index(), h_b.origin.index());
                    assert_ne!(h_a.incident_face, h_b.incident_face);
                }
                count => {
                    panic!("Non-manifold edge between vertex {u0} and {u1}: count = {count}");
                }
            }
        }

        assert_eq!(paired_count, surface.stats.paired_half_edge_count);
        assert_eq!(boundary_count, surface.stats.boundary_half_edge_count);
        assert_eq!(paired_count + boundary_count, surface.half_edges.len());
    }
}
