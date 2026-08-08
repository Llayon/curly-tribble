// src/map/surface_topology/tests_compatibility.rs
//! Bit-exact compatibility proof between `SurfaceTopology` and direct `derive_terrain_topology` adapter.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTopologyCompatibilityTestsPlugin;

impl Plugin for SurfaceTopologyCompatibilityTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::topology::derive_terrain_topology;
    use crate::map::HexCoord;

    #[test]
    fn direct_adapter_bit_exact_compatibility() {
        let mut map = MapData::default();
        for q in 0..3 {
            for r in 0..3 {
                map.tiles.insert(HexCoord::new(q, r), TileData::default());
            }
        }

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Face topology failed");

        let direct = derive_terrain_topology(&map, &face_topology).expect("Direct adapter failed");
        let surface = generate_surface_topology(&face_topology).expect("Surface generator failed");

        assert_eq!(surface.vertices.len(), direct.vertices_xz.len());
        assert_eq!(surface.faces.len(), direct.triangles.len());
        assert_eq!(surface.faces.len(), direct.triangle_cells.len());

        for i in 0..surface.vertices.len() {
            let s_pos = surface.vertices[i].position;
            let d_pos = direct.vertices_xz[i];
            assert_eq!(
                s_pos.x.to_bits(),
                d_pos.x.to_bits(),
                "Vertex {i} X position mismatch"
            );
            assert_eq!(
                s_pos.y.to_bits(),
                d_pos.y.to_bits(),
                "Vertex {i} Y position mismatch"
            );
        }

        for i in 0..surface.faces.len() {
            let s_face = &surface.faces[i];
            let d_tri = direct.triangles[i];
            let d_cell = direct.triangle_cells[i];

            assert_eq!(
                s_face.vertices[0].index(),
                d_tri[0] as usize,
                "Face {i} v0 mismatch"
            );
            assert_eq!(
                s_face.vertices[1].index(),
                d_tri[1] as usize,
                "Face {i} v1 mismatch"
            );
            assert_eq!(
                s_face.vertices[2].index(),
                d_tri[2] as usize,
                "Face {i} v2 mismatch"
            );
            assert_eq!(s_face.owner_hex, d_cell, "Face {i} owner_hex mismatch");
        }
    }
}
