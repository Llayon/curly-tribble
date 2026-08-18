// src/map/terrain_bake/tests_render.rs
//! Renderer-level cliff wall proof: the M5.1 bake renderer
//! (`create_global_map_meshes_from_bake`) must emit exactly:
//! - full cliff segment → 2 wall triangles
//! - start taper        → 1 wall triangle
//! - end taper          → 1 wall triangle
//! - equal heights      → 0 wall triangles
//! - flat phase         → 0 wall triangles
//!
//! Every wall index must be `>= ground_vertex_count` and `< vertex_count`;
//! all positions and normals must be finite.

#[cfg(test)]
pub mod tests {
    use crate::economy::mesh_gen::bake::create_global_map_meshes_from_bake;
    use crate::game_state::{EditorPhase, FactionManager};
    use crate::map::data::{MapData, OceanState, TileData};
    use crate::map::face_topology::types::HexFaceTopology;
    use crate::map::surface_height::types::SurfaceHeightLayer;
    use crate::map::terrain_bake::builder::build_surface_terrain_bake;
    use crate::map::terrain_bake::tests_walls::tests::build_two_hex_cliff_surface;
    use crate::map::terrain_bake::types::SurfaceTerrainBake;
    use crate::map::terrain_gen::TerrainConfig;
    use crate::map::{HexCoord, MAX_HEIGHT};
    use bevy::mesh::Indices;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct TerrainBakeRenderTestsPlugin;

    impl Plugin for TerrainBakeRenderTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// Builds the two-hex cliff bake with the given per-node heights and renders it.
    /// Returns (terrain mesh, ground_vertex_count, bake).
    fn render_two_hex_bake(
        heights: [f32; 6],
        phase: EditorPhase,
    ) -> (Mesh, usize, SurfaceTerrainBake) {
        let (surface, graph) = build_two_hex_cliff_surface(0.0, 0.0);
        let mut layer = SurfaceHeightLayer::default();
        layer.heights = heights.to_vec();
        let bake = build_surface_terrain_bake(&surface, &graph, &layer)
            .expect("two-hex cliff bake must build");

        let mut map = MapData::default();
        for (q, r) in [(0, 0), (1, 0)] {
            map.tiles.insert(
                HexCoord::new(q, r),
                TileData {
                    ocean_state: OceanState::Land,
                    elevation: 0.5,
                    ..Default::default()
                },
            );
        }

        let (mesh, _, _) = create_global_map_meshes_from_bake(
            &map,
            &bake,
            &HexFaceTopology::default(),
            phase,
            &FactionManager::default(),
            &TerrainConfig::default(),
            &crate::map::surface_gameplay::types::SurfaceGameplayMap::default(),
        )
        .expect("bake render must succeed");

        (mesh, bake.vertices.len(), bake)
    }

    /// Splits mesh indices into ground part (bake faces) and wall part.
    /// Asserts the ground part is exactly the bake face node ids, and returns
    /// (wall_triangle_count, wall_indices).
    fn split_wall_indices(
        mesh: &Mesh,
        bake: &SurfaceTerrainBake,
        ground_vertex_count: usize,
    ) -> (usize, Vec<u32>) {
        let Some(Indices::U32(idx)) = mesh.indices() else {
            panic!("terrain mesh must have U32 indices");
        };

        let ground_index_count = bake.faces.len() * 3;
        assert!(
            idx.len() >= ground_index_count,
            "mesh must contain at least the ground face indices"
        );
        for (k, face) in bake.faces.iter().enumerate() {
            for (c, &node_id) in face.nodes.iter().enumerate() {
                assert_eq!(
                    idx[k * 3 + c],
                    node_id.index() as u32,
                    "ground triangle {k} corner {c} must be the face's node id"
                );
            }
        }

        let wall_indices = idx[ground_index_count..].to_vec();
        assert_eq!(
            wall_indices.len() % 3,
            0,
            "wall indices must be a whole number of triangles"
        );
        (wall_indices.len() / 3, wall_indices)
    }

    /// Asserts every position/normal is finite and every index is in range.
    fn assert_mesh_geometry_sane(mesh: &Mesh, ground_vertex_count: usize) {
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("missing position attribute");
        };
        for (i, p) in positions.iter().enumerate() {
            assert!(
                p.iter().all(|c| c.is_finite()),
                "vertex {i}: position must be finite, got {p:?}"
            );
        }

        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("missing normal attribute");
        };
        assert_eq!(normals.len(), positions.len(), "normal count must match");
        for (i, n) in normals.iter().enumerate() {
            assert!(
                n.iter().all(|c| c.is_finite()),
                "vertex {i}: normal must be finite, got {n:?}"
            );
        }

        let Some(Indices::U32(idx)) = mesh.indices() else {
            panic!("missing U32 indices");
        };
        for &i in idx {
            assert!(
                (i as usize) < positions.len(),
                "index {i} out of range for {} vertices",
                positions.len()
            );
        }
    }

    /// Case 1: full cliff — both endpoints split → exactly 2 wall triangles.
    #[test]
    fn full_cliff_renders_two_wall_triangles() {
        let (mesh, ground_vertex_count, bake) =
            render_two_hex_bake([0.1, 0.9, 0.1, 0.9, 0.3, 0.3], EditorPhase::Height3D);

        let (wall_triangles, wall_indices) = split_wall_indices(&mesh, &bake, ground_vertex_count);
        assert_eq!(wall_triangles, 2, "full cliff must emit 2 wall triangles");

        for &i in &wall_indices {
            assert!(
                (i as usize) >= ground_vertex_count,
                "wall index {i} must be >= ground_vertex_count {ground_vertex_count}"
            );
        }
        assert_mesh_geometry_sane(&mesh, ground_vertex_count);
    }

    /// Case 2: start taper — origin endpoint differs, destination equal → 1 triangle.
    #[test]
    fn start_taper_renders_one_wall_triangle() {
        let (mesh, ground_vertex_count, bake) =
            render_two_hex_bake([0.1, 0.9, 0.5, 0.5, 0.3, 0.3], EditorPhase::Height3D);

        let (wall_triangles, _) = split_wall_indices(&mesh, &bake, ground_vertex_count);
        assert_eq!(wall_triangles, 1, "start taper must emit 1 wall triangle");
        assert_mesh_geometry_sane(&mesh, ground_vertex_count);
    }

    /// Case 3: end taper — origin equal, destination endpoint differs → 1 triangle.
    #[test]
    fn end_taper_renders_one_wall_triangle() {
        let (mesh, ground_vertex_count, bake) =
            render_two_hex_bake([0.5, 0.5, 0.1, 0.9, 0.3, 0.3], EditorPhase::Height3D);

        let (wall_triangles, _) = split_wall_indices(&mesh, &bake, ground_vertex_count);
        assert_eq!(wall_triangles, 1, "end taper must emit 1 wall triangle");
        assert_mesh_geometry_sane(&mesh, ground_vertex_count);
    }

    /// Case 4: equal heights on both endpoints → 0 wall triangles (degenerate).
    #[test]
    fn equal_heights_render_no_wall_triangles() {
        let (mesh, ground_vertex_count, bake) =
            render_two_hex_bake([0.5, 0.5, 0.5, 0.5, 0.3, 0.3], EditorPhase::Height3D);

        let (wall_triangles, _) = split_wall_indices(&mesh, &bake, ground_vertex_count);
        assert_eq!(
            wall_triangles, 0,
            "equal heights must emit 0 wall triangles"
        );
        assert_mesh_geometry_sane(&mesh, ground_vertex_count);
    }

    /// Case 5: flat phase — walls are skipped entirely even for full cliffs.
    #[test]
    fn flat_phase_renders_no_wall_triangles() {
        let (mesh, ground_vertex_count, bake) =
            render_two_hex_bake([0.1, 0.9, 0.1, 0.9, 0.3, 0.3], EditorPhase::Shape);

        let (wall_triangles, _) = split_wall_indices(&mesh, &bake, ground_vertex_count);
        assert_eq!(wall_triangles, 0, "flat phase must emit 0 wall triangles");

        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("missing position attribute");
        };
        assert_eq!(
            positions.len(),
            ground_vertex_count,
            "flat: no wall vertices"
        );
        for p in positions {
            assert_eq!(p[1], 0.0, "flat: every Y must be 0");
        }
    }

    /// Case 6: relief Y is normalized_height * MAX_HEIGHT on the render boundary.
    #[test]
    fn relief_y_scales_by_max_height() {
        let (mesh, _ground_vertex_count, bake) =
            render_two_hex_bake([0.1, 0.9, 0.1, 0.9, 0.3, 0.3], EditorPhase::Height3D);

        let Some(bevy::mesh::VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("missing position attribute");
        };
        for (i, v) in bake.vertices.iter().enumerate() {
            assert_eq!(
                positions[i][1],
                v.normalized_height * MAX_HEIGHT,
                "ground vertex {i} Y must be normalized_height * MAX_HEIGHT"
            );
        }
    }
}
