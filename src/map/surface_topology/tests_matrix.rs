// src/map/surface_topology/tests_matrix.rs
//! Canonical 144-case proof matrix and extended stress matrix tests for SurfaceTopology.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTopologyMatrixTestsPlugin;

impl Plugin for SurfaceTopologyMatrixTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::WorldSeed;
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::surface_topology::types::SurfaceVertexSource;
    use crate::map::surface_topology::validation::validate_surface_topology;

    #[test]
    fn canonical_144_case_surface_topology_matrix() {
        let mut cases = 0;
        let mut total_surface_faces = 0;
        let mut total_surface_half_edges = 0;

        for (shape, map) in q::all_shapes() {
            for seed_val in q::FAST_SEEDS {
                for profile in q::all_profiles() {
                    cases += 1;
                    let seed = WorldSeed::new(seed_val);
                    let face_topology =
                        generate_hex_face_topology_with_profile(&map, seed, profile)
                            .expect("Face topology generation failed");

                    let surface = generate_surface_topology(&face_topology)
                        .expect("Surface topology generation failed");

                    validate_surface_topology(&surface).expect("Surface validation failed");

                    assert_eq!(
                        surface.faces.len(),
                        map.tiles.len() * 24,
                        "Shape {shape} seed {seed_val} profile {profile:?}: face count mismatch"
                    );
                    assert_eq!(
                        surface.half_edges.len(),
                        surface.faces.len() * 3,
                        "Shape {shape} seed {seed_val} profile {profile:?}: half-edge count mismatch"
                    );

                    total_surface_faces += surface.faces.len();
                    total_surface_half_edges += surface.half_edges.len();

                    for vertex in &surface.vertices {
                        assert!(vertex.position.is_finite());
                        if let SurfaceVertexSource::HexCorner { source_vertex } = &vertex.source {
                            let source_pos = face_topology.vertices[source_vertex.index()].position;
                            assert_eq!(
                                (vertex.position.x.to_bits(), vertex.position.y.to_bits()),
                                (source_pos.x.to_bits(), source_pos.y.to_bits())
                            );
                        }
                    }

                    if shape == "isolated" {
                        assert_eq!(
                            face_topology.stats.paired_edge_count, 0,
                            "isolated coarse shape must have 0 paired edges"
                        );
                        assert!(
                            surface.stats.paired_half_edge_count > 0,
                            "isolated cell must have interior paired half-edges"
                        );
                    }
                }
            }
        }

        assert_eq!(cases, 144);
        assert!(total_surface_faces > 0);
        assert!(total_surface_half_edges > 0);
    }

    #[test]
    #[ignore]
    fn surface_topology_extended_4608_matrix() {
        let mut cases = 0;
        let mut total_surface_faces = 0;
        let mut total_surface_half_edges = 0;

        for (_shape, map) in q::all_shapes() {
            for seed_val in 0..256 {
                for profile in q::all_profiles() {
                    cases += 1;
                    let seed = WorldSeed::new(seed_val);
                    let face_topology =
                        generate_hex_face_topology_with_profile(&map, seed, profile)
                            .expect("Face topology failed");

                    let surface =
                        generate_surface_topology(&face_topology).expect("Surface topology failed");

                    validate_surface_topology(&surface).expect("Surface validation failed");

                    total_surface_faces += surface.faces.len();
                    total_surface_half_edges += surface.half_edges.len();
                }
            }
        }

        assert_eq!(cases, 4608);
        assert!(total_surface_faces > 0);
        assert!(total_surface_half_edges > 0);
    }
}
