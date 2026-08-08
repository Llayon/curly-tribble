// src/map/surface_topology/tests_terrain_adapter.rs
//! Bit-compatibility proof and unit tests for `derive_terrain_topology_from_surface`.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTerrainAdapterTestsPlugin;

impl Plugin for SurfaceTerrainAdapterTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::surface_topology::terrain_adapter::derive_terrain_topology_from_surface;
    use crate::map::surface_topology::types::{SurfaceTopology, SurfaceVertexSource};
    use crate::map::topology::derive_terrain_topology;
    use crate::map::HexCoord;
    use bevy::app::App;
    use bevy::MinimalPlugins;

    #[test]
    fn surface_to_terrain_144_case_matrix() {
        let mut cases = 0;

        for (shape, map) in q::all_shapes() {
            for seed_val in q::FAST_SEEDS {
                for profile in q::all_profiles() {
                    cases += 1;
                    let seed = WorldSeed::new(seed_val);
                    let face_topology =
                        generate_hex_face_topology_with_profile(&map, seed, profile)
                            .expect("Face topology failed");

                    let surface =
                        generate_surface_topology(&face_topology).expect("Surface topology failed");

                    let direct_terrain = derive_terrain_topology(&map, &face_topology)
                        .expect("Direct adapter failed");
                    let surface_terrain = derive_terrain_topology_from_surface(&surface)
                        .expect("Surface terrain adapter failed");

                    assert_eq!(
                        surface_terrain.vertices_xz.len(),
                        direct_terrain.vertices_xz.len(),
                        "Shape {shape} seed {seed_val}: vertex count mismatch"
                    );
                    for (i, (s_v, d_v)) in surface_terrain
                        .vertices_xz
                        .iter()
                        .zip(&direct_terrain.vertices_xz)
                        .enumerate()
                    {
                        assert_eq!(
                            (s_v.x.to_bits(), s_v.y.to_bits()),
                            (d_v.x.to_bits(), d_v.y.to_bits()),
                            "Shape {shape} seed {seed_val}: vertex {i} position mismatch"
                        );
                    }

                    assert_eq!(
                        surface_terrain.triangles, direct_terrain.triangles,
                        "Shape {shape} seed {seed_val}: triangles mismatch"
                    );
                    assert_eq!(
                        surface_terrain.triangle_cells, direct_terrain.triangle_cells,
                        "Shape {shape} seed {seed_val}: triangle_cells mismatch"
                    );
                    assert_eq!(
                        surface_terrain.vertex_influences, direct_terrain.vertex_influences,
                        "Shape {shape} seed {seed_val}: vertex_influences mismatch"
                    );
                }
            }
        }

        assert_eq!(cases, 144);
    }

    #[test]
    #[ignore]
    fn surface_to_terrain_extended_4608_matrix() {
        let mut cases = 0;

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

                    let direct_terrain = derive_terrain_topology(&map, &face_topology)
                        .expect("Direct adapter failed");
                    let surface_terrain = derive_terrain_topology_from_surface(&surface)
                        .expect("Surface terrain adapter failed");

                    assert_eq!(
                        surface_terrain.vertices_xz.len(),
                        direct_terrain.vertices_xz.len()
                    );
                    for (s_v, d_v) in surface_terrain
                        .vertices_xz
                        .iter()
                        .zip(&direct_terrain.vertices_xz)
                    {
                        assert_eq!(
                            (s_v.x.to_bits(), s_v.y.to_bits()),
                            (d_v.x.to_bits(), d_v.y.to_bits())
                        );
                    }
                    assert_eq!(surface_terrain.triangles, direct_terrain.triangles);
                    assert_eq!(
                        surface_terrain.triangle_cells,
                        direct_terrain.triangle_cells
                    );
                    assert_eq!(
                        surface_terrain.vertex_influences,
                        direct_terrain.vertex_influences
                    );
                }
            }
        }

        assert_eq!(cases, 4608);
    }

    #[test]
    fn surface_terrain_adapter_focused_influence_semantics() {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Face topology failed");
        let surface = generate_surface_topology(&face_topology).expect("Surface failed");
        let terrain =
            derive_terrain_topology_from_surface(&surface).expect("Terrain adapter failed");

        for (idx, vertex) in surface.vertices.iter().enumerate() {
            let influences = &terrain.vertex_influences[idx];
            assert!(!influences.is_empty());
            assert!(influences
                .windows(2)
                .all(|w| (w[0].q, w[0].r) <= (w[1].q, w[1].r)));

            match &vertex.source {
                SurfaceVertexSource::HexCenter { hex } => {
                    assert_eq!(influences, &vec![*hex]);
                }
                SurfaceVertexSource::HexRadialMidpoint { hex, .. } => {
                    assert_eq!(influences, &vec![*hex]);
                }
                SurfaceVertexSource::HexCorner { .. }
                | SurfaceVertexSource::HexEdgeMidpoint { .. } => {
                    assert!(influences.len() <= 2);
                }
            }
        }
    }

    #[test]
    fn surface_terrain_adapter_empty_surface_returns_default() {
        let empty_surface = SurfaceTopology::default();
        let terrain = derive_terrain_topology_from_surface(&empty_surface)
            .expect("Empty surface projects to default terrain");
        assert!(terrain.vertices_xz.is_empty());
        assert!(terrain.triangles.is_empty());
        assert!(terrain.triangle_cells.is_empty());
        assert!(terrain.vertex_influences.is_empty());
    }

    #[test]
    fn same_frame_topology_regeneration_ordering() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::face_topology::FaceTopologyPlugin,
            crate::map::surface_topology::SurfaceTopologyPlugin,
        ));
        app.add_message::<crate::map::GenerateMapEvent>()
            .add_message::<crate::map::RebuildMeshEvent>()
            .insert_resource(MapData::default())
            .insert_resource(WorldSeed::new(42));

        app.update();
        let initial_gen = app
            .world()
            .resource::<crate::map::surface_topology::runtime::SurfaceTopologyGenerationState>()
            .generation_count;

        let mut map = app.world_mut().resource_mut::<MapData>();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());

        app.update();
        let updated_gen = app
            .world()
            .resource::<crate::map::surface_topology::runtime::SurfaceTopologyGenerationState>()
            .generation_count;
        assert_eq!(updated_gen, initial_gen + 1);

        let surface = app.world().resource::<SurfaceTopology>();
        assert!(!surface.vertices.is_empty());
    }

    #[test]
    fn elevation_only_rebuild_does_not_regenerate_surface() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::face_topology::FaceTopologyPlugin,
            crate::map::surface_topology::SurfaceTopologyPlugin,
        ));
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());
        app.add_message::<crate::map::GenerateMapEvent>()
            .add_message::<crate::map::RebuildMeshEvent>()
            .insert_resource(map)
            .insert_resource(WorldSeed::new(42));

        app.update();
        let initial_surface_gen = app
            .world()
            .resource::<crate::map::surface_topology::runtime::SurfaceTopologyGenerationState>()
            .generation_count;
        let initial_coarse_gen = app
            .world()
            .resource::<crate::map::face_topology::runtime::HexFaceTopologyGenerationState>()
            .generation_count;

        let mut map = app.world_mut().resource_mut::<MapData>();
        if let Some(tile) = map.tiles.get_mut(&HexCoord::new(0, 0)) {
            tile.elevation = 5.0;
        }

        app.update();
        let updated_surface_gen = app
            .world()
            .resource::<crate::map::surface_topology::runtime::SurfaceTopologyGenerationState>()
            .generation_count;
        let updated_coarse_gen = app
            .world()
            .resource::<crate::map::face_topology::runtime::HexFaceTopologyGenerationState>()
            .generation_count;

        assert_eq!(updated_surface_gen, initial_surface_gen);
        assert_eq!(updated_coarse_gen, initial_coarse_gen);
    }

    #[test]
    fn empty_map_transition_clears_surface_terrain() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            crate::map::face_topology::FaceTopologyPlugin,
            crate::map::surface_topology::SurfaceTopologyPlugin,
        ));
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());
        app.add_message::<crate::map::GenerateMapEvent>()
            .add_message::<crate::map::RebuildMeshEvent>()
            .insert_resource(map)
            .insert_resource(WorldSeed::new(42));

        app.update();
        assert!(!app
            .world()
            .resource::<SurfaceTopology>()
            .vertices
            .is_empty());

        let mut map = app.world_mut().resource_mut::<MapData>();
        map.tiles.clear();

        app.update();
        let surface = app.world().resource::<SurfaceTopology>();
        assert!(surface.vertices.is_empty());

        let terrain =
            derive_terrain_topology_from_surface(surface).expect("Default empty projection");
        assert!(terrain.vertices_xz.is_empty());
    }
}
