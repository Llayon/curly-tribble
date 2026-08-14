// src/map/terrain_bake/tests_builder.rs
//! Canonical 144-case bake matrix and 40×40 production smoke gate.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{MapData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::height_constraints::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::surface_height::guide::derive_legacy_height_guide;
    use crate::map::surface_height::hard_constraints::compile_hard_constraints;
    use crate::map::surface_height::solver::solve_surface_heights;
    use crate::map::surface_height::targets::compile_height_targets;
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::surface_height::validation::validate_surface_height_layer;
    use crate::map::surface_topology::generate_surface_topology;
    use crate::map::terrain_bake::builder::build_surface_terrain_bake;
    use crate::map::terrain_bake::validation::validate_surface_terrain_bake;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct TerrainBakeBuilderTestsPlugin;

    impl Plugin for TerrainBakeBuilderTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// Full canonical 144-case bake matrix:
    /// 6 shapes × 3 profiles × 8 seeds → build + validate for each.
    #[test]
    fn canonical_144_bake_matrix() {
        let config = HeightSolverConfig::default();
        let mut cases = 0;

        for (_shape, map) in q::all_shapes() {
            for profile in q::all_profiles() {
                for &seed_val in &q::FAST_SEEDS {
                    cases += 1;
                    let seed = WorldSeed::new(seed_val);
                    let face_top =
                        generate_hex_face_topology_with_profile(&map, seed, profile).unwrap();
                    let surface = generate_surface_topology(&face_top).unwrap();
                    let constraints = compile_height_constraints(&map, &surface).unwrap();
                    let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
                    let guide = derive_legacy_height_guide(&map, &surface, &graph).unwrap();
                    let targets = compile_height_targets(&graph, &guide, &config).unwrap();
                    let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
                    let layer =
                        solve_surface_heights(&graph, &guide, &targets, &hard, &config).unwrap();
                    validate_surface_height_layer(&layer, &graph, &guide, &hard, &config).unwrap();

                    let bake =
                        build_surface_terrain_bake(&surface, &graph, &layer).unwrap_or_else(|e| {
                            panic!("bake build failed case {cases} seed={seed_val}: {e:?}")
                        });
                    validate_surface_terrain_bake(&bake, &surface, &graph, &layer).unwrap_or_else(
                        |e| panic!("bake validation failed case {cases} seed={seed_val}: {e:?}"),
                    );

                    // Structural invariants
                    assert_eq!(
                        bake.vertices.len(),
                        graph.nodes.len(),
                        "case {cases}: vertex count must equal node count"
                    );
                    assert_eq!(
                        bake.faces.len(),
                        surface.faces.len(),
                        "case {cases}: face count must equal surface face count"
                    );

                    // XZ bit-exact with source
                    for (i, v) in bake.vertices.iter().enumerate() {
                        let src = &surface.vertices[graph.nodes[i].surface_vertex.index()];
                        assert_eq!(
                            v.position_xz.x.to_bits(),
                            src.position.x.to_bits(),
                            "case {cases} node {i}: XZ x mismatch"
                        );
                        assert_eq!(
                            v.position_xz.y.to_bits(),
                            src.position.y.to_bits(),
                            "case {cases} node {i}: XZ y mismatch"
                        );
                        // Height bit-exact with source
                        assert_eq!(
                            v.normalized_height.to_bits(),
                            layer.heights[i].to_bits(),
                            "case {cases} node {i}: height mismatch"
                        );
                    }
                }
            }
        }

        assert_eq!(cases, 144, "Expected 144 cases");
    }

    /// Empty bake contract: all-empty inputs → empty bake, no error.
    #[test]
    fn empty_bake_returns_default() {
        use crate::map::height_graph::types::HeightConstraintGraph;
        use crate::map::surface_height::types::SurfaceHeightLayer;
        use crate::map::surface_topology::types::SurfaceTopology;

        let bake = build_surface_terrain_bake(
            &SurfaceTopology::default(),
            &HeightConstraintGraph::default(),
            &SurfaceHeightLayer::default(),
        )
        .expect("empty bake");
        assert!(bake.vertices.is_empty());
        assert!(bake.faces.is_empty());
        assert!(bake.cliff_walls.is_empty());
    }

    /// 40×40 production smoke: full pipeline → bake builds and validates.
    #[test]
    fn production_40x40_bake_smoke() {
        use crate::game_state::EditorPhase;
        use crate::map::generation::terrain::spawn_map_internal;
        use crate::map::navigation::NavigationMap;
        use crate::map::terrain_gen::{TerrainConfig, TerrainGenerator};
        use crate::map::GenerationMode;

        let terrain_config = TerrainConfig::default();
        let config = HeightSolverConfig::default();

        for &seed_val in &q::FAST_SEEDS {
            let terrain_gen = TerrainGenerator::new(seed_val);
            let seed = WorldSeed::new(seed_val);
            let mut map_data = MapData::default();
            let mut nav_map = NavigationMap::default();

            spawn_map_internal(
                &terrain_gen,
                &terrain_config,
                &seed,
                &mut map_data,
                &mut nav_map,
                EditorPhase::Landscape,
                GenerationMode::Reset,
                Some(EditorPhase::Landscape),
            );

            let face_top = generate_hex_face_topology_with_profile(
                &map_data,
                seed,
                crate::map::face_topology::profiles::HexDeformationProfile::Organic,
            )
            .unwrap();
            let surface = generate_surface_topology(&face_top).unwrap();
            let constraints = compile_height_constraints(&map_data, &surface).unwrap();
            let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
            let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();
            let targets = compile_height_targets(&graph, &guide, &config).unwrap();
            let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
            let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config)
                .unwrap_or_else(|e| panic!("solver failed seed={seed_val}: {e:?}"));
            validate_surface_height_layer(&layer, &graph, &guide, &hard, &config).unwrap();

            let bake = build_surface_terrain_bake(&surface, &graph, &layer)
                .unwrap_or_else(|e| panic!("bake failed seed={seed_val}: {e:?}"));
            validate_surface_terrain_bake(&bake, &surface, &graph, &layer)
                .unwrap_or_else(|e| panic!("bake validation failed seed={seed_val}: {e:?}"));

            assert!(
                !bake.vertices.is_empty(),
                "seed={seed_val}: bake must be non-empty"
            );
            assert!(
                !bake.faces.is_empty(),
                "seed={seed_val}: bake faces must be non-empty"
            );

            // Heights all in [0,1]
            for v in &bake.vertices {
                assert!(
                    (0.0..=1.0).contains(&v.normalized_height),
                    "seed={seed_val}: normalized_height out of range"
                );
            }
        }
    }
}
