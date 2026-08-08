// src/map/surface_topology/tests_shared_boundary.rs
//! Two-cell coarse shared-boundary surface topology proof.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTopologySharedBoundaryTestsPlugin;

impl Plugin for SurfaceTopologySharedBoundaryTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::surface_topology::types::SurfaceVertexSource;
    use crate::map::HexCoord;

    #[test]
    fn two_cell_shared_boundary_proof() {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Face topology generation failed");

        let surface =
            generate_surface_topology(&face_topology).expect("Surface topology generation failed");

        let faces_c1 = surface.hex_to_faces.get(&c1).expect("c1 faces");
        let faces_c2 = surface.hex_to_faces.get(&c2).expect("c2 faces");

        assert_eq!(faces_c1.len(), 24);
        assert_eq!(faces_c2.len(), 24);

        // Find the coarse shared edge vertices
        let face1 = face_topology
            .faces
            .get(face_topology.hex_to_face[&c1].index())
            .unwrap();
        let face2 = face_topology
            .faces
            .get(face_topology.hex_to_face[&c2].index())
            .unwrap();

        let shared_coarse_verts: Vec<_> = face1
            .vertices
            .iter()
            .copied()
            .filter(|v| face2.vertices.contains(v))
            .collect();

        assert_eq!(shared_coarse_verts.len(), 2);
        let source_a = shared_coarse_verts[0].min(shared_coarse_verts[1]);
        let source_b = shared_coarse_verts[0].max(shared_coarse_verts[1]);

        // Find corresponding SurfaceVertexIds
        let mut surface_v_a = None;
        let mut surface_v_b = None;
        let mut surface_v_mid = None;

        for (idx, vertex) in surface.vertices.iter().enumerate() {
            let s_id = crate::map::surface_topology::types::SurfaceVertexId::new(idx);
            match &vertex.source {
                SurfaceVertexSource::HexCorner { source_vertex } => {
                    if *source_vertex == source_a {
                        surface_v_a = Some(s_id);
                    } else if *source_vertex == source_b {
                        surface_v_b = Some(s_id);
                    }
                }
                SurfaceVertexSource::HexEdgeMidpoint {
                    source_a: sa,
                    source_b: sb,
                } => {
                    if *sa == source_a && *sb == source_b {
                        surface_v_mid = Some(s_id);
                    }
                }
                _ => {}
            }
        }

        let sv_a = surface_v_a.expect("Surface corner A");
        let sv_b = surface_v_b.expect("Surface corner B");
        let sv_mid = surface_v_mid.expect("Surface edge midpoint");

        // Verify shared half-edges across c1 and c2
        let seg1_indices: Vec<_> = surface
            .half_edges
            .iter()
            .enumerate()
            .filter(|(_, he)| {
                (he.origin == sv_a && he.destination == sv_mid)
                    || (he.origin == sv_mid && he.destination == sv_a)
            })
            .map(|(idx, he)| {
                (
                    crate::map::surface_topology::types::SurfaceHalfEdgeId::new(idx),
                    he,
                )
            })
            .collect();

        assert_eq!(seg1_indices.len(), 2);
        let (h1_id, h1_he) = seg1_indices[0];
        let (h2_id, h2_he) = seg1_indices[1];
        assert_eq!(h1_he.twin, Some(h2_id));
        assert_eq!(h2_he.twin, Some(h1_id));

        let seg2_indices: Vec<_> = surface
            .half_edges
            .iter()
            .enumerate()
            .filter(|(_, he)| {
                (he.origin == sv_mid && he.destination == sv_b)
                    || (he.origin == sv_b && he.destination == sv_mid)
            })
            .map(|(idx, he)| {
                (
                    crate::map::surface_topology::types::SurfaceHalfEdgeId::new(idx),
                    he,
                )
            })
            .collect();

        assert_eq!(seg2_indices.len(), 2);
        let (h3_id, h3_he) = seg2_indices[0];
        let (h4_id, h4_he) = seg2_indices[1];
        assert_eq!(h3_he.twin, Some(h4_id));
        assert_eq!(h4_he.twin, Some(h3_id));
    }
}
