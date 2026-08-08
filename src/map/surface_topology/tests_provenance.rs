// src/map/surface_topology/tests_provenance.rs
//! Provenance and corner identity unit tests.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTopologyProvenanceTestsPlugin;

impl Plugin for SurfaceTopologyProvenanceTestsPlugin {
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
    fn corner_edge_center_radial_provenance_integrity() {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Organic)
                .expect("Face topology failed");
        let surface = generate_surface_topology(&face_topology).expect("Surface topology failed");

        let mut corner_count = 0;
        let mut center_count = 0;
        let mut radial_count = 0;
        let mut edge_count = 0;

        for vertex in &surface.vertices {
            match &vertex.source {
                SurfaceVertexSource::HexCorner { source_vertex } => {
                    corner_count += 1;
                    let source_pos = face_topology.vertices[source_vertex.index()].position;
                    assert_eq!(
                        (vertex.position.x.to_bits(), vertex.position.y.to_bits()),
                        (source_pos.x.to_bits(), source_pos.y.to_bits())
                    );
                }
                SurfaceVertexSource::HexCenter { hex } => {
                    center_count += 1;
                    assert!(hex == &c1 || hex == &c2);
                }
                SurfaceVertexSource::HexRadialMidpoint { hex, source_corner } => {
                    radial_count += 1;
                    assert!(hex == &c1 || hex == &c2);
                    assert!(source_corner.index() < face_topology.vertices.len());
                }
                SurfaceVertexSource::HexEdgeMidpoint { source_a, source_b } => {
                    edge_count += 1;
                    assert!(source_a.index() < source_b.index());
                }
            }
        }

        assert_eq!(corner_count, face_topology.vertices.len());
        assert_eq!(center_count, 2);
        assert_eq!(radial_count, 12);
        assert_eq!(
            edge_count,
            face_topology.stats.paired_edge_count + face_topology.stats.border_edge_count
        );
    }
}
