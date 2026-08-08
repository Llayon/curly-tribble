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
        assert!(surface.stats.paired_half_edge_count > 0);
        assert!(surface.stats.boundary_half_edge_count > 0);
        assert_eq!(
            surface.stats.paired_half_edge_count + surface.stats.boundary_half_edge_count,
            surface.half_edges.len()
        );
    }
}
