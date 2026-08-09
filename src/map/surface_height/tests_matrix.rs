// src/map/surface_height/tests_matrix.rs
//! Canonical 144-case geometry-independence proof, semantic 144-case surface height matrix,
//! and production 40x40 smoke gate. Synthetic 4,608 matrix is in tests_matrix_synthetic.rs.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{
        CliffLowerSide, EdgeCoord, EdgeData, EdgeType, LandscapeFeature, MapData, OceanState,
        TileData, WorldSeed,
    };
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::height_constraints::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::height_graph::types::{
        CliffNodeRelation, HeightConstraintGraph, HeightContinuityEdge, HeightNode, HeightNodeId,
    };
    use crate::map::surface_height::guide::{
        derive_legacy_height_guide, HeightGuideSample, LegacyHeightGuide,
    };
    use crate::map::surface_height::hard_constraints::compile_hard_constraints;
    use crate::map::surface_height::solver::solve_surface_heights;
    use crate::map::surface_height::targets::compile_height_targets;
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::surface_height::validation::validate_surface_height_layer;
    use crate::map::surface_topology::generate_surface_topology;
    use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceVertexId};
    use crate::map::HexCoord;
    use bevy::prelude::*;
    use std::collections::HashMap;

    #[allow(dead_code)]
    pub struct SurfaceHeightMatrixTestsPlugin;

    impl Plugin for SurfaceHeightMatrixTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// True production feasibility gate: runs spawn_map_internal with full landscape generation
    /// (Mountains, Plateaus, Lakes, generate_cliffs, apply_rivers) over 40×40 × FAST_SEEDS.
    #[test]
    fn production_default_landscape_is_solver_feasible() {
        use crate::game_state::EditorPhase;
        use crate::map::generation::terrain::spawn_map_internal;
        use crate::map::navigation::NavigationMap;
        use crate::map::terrain_gen::{TerrainConfig, TerrainGenerator};
        use crate::map::GenerationMode;

        // TerrainConfig::default() sets map_width = 40, map_height = 40.
        let terrain_config = TerrainConfig::default();

        for &seed_val in &q::FAST_SEEDS {
            // TerrainGenerator has no Default impl — must use ::new(seed).
            let terrain_gen = TerrainGenerator::new(seed_val);
            let seed = WorldSeed::new(seed_val);
            let mut map_data = MapData::default();
            let mut nav_map = NavigationMap::default();

            // Full production generation: apply_landscape_generation + generate_cliffs inside.
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
                HexDeformationProfile::Organic,
            )
            .unwrap();
            let surface = generate_surface_topology(&face_top).unwrap();
            let constraints = compile_height_constraints(&map_data, &surface).unwrap();
            let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
            let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();

            let config = HeightSolverConfig::default(); // cliff_min_drop = 0.10
            let targets = compile_height_targets(&graph, &guide, &config).unwrap();
            let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
            let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config)
                .unwrap_or_else(|e| {
                    panic!(
                        "production 40x40 landscape seed={seed_val} must be solver-feasible: {e:?}"
                    )
                });

            validate_surface_height_layer(&layer, &graph, &guide, &hard, &config).unwrap();
            assert_eq!(layer.heights.len(), graph.nodes.len());
            // Production world generates real cliffs; assert constraints when cliffs exist.
            if !hard.edges.is_empty() {
                assert!(layer.stats.resolved_cliff_constraint_count > 0);
            }
        }
    }

    // ─── Canonical 144 — geometry independence ───────────────────────────────

    /// Proof: 6 shapes × 3 profiles × 8 seeds produce bit-exact height layers per shape/seed,
    /// proving solver output is geometry-independent (no face-corner randomness bleeds through).
    #[test]
    fn canonical_144_geometry_independence() {
        let config = HeightSolverConfig::default();

        for (_shape_name, map_template) in q::all_shapes() {
            let mut baseline_bits: Option<Vec<u32>> = None;

            for profile in q::all_profiles() {
                for seed_val in q::FAST_SEEDS {
                    let seed = WorldSeed::new(seed_val);
                    let map_data = map_template.clone();

                    let face_top =
                        generate_hex_face_topology_with_profile(&map_data, seed, profile).unwrap();
                    let surface = generate_surface_topology(&face_top).unwrap();
                    let constraints = compile_height_constraints(&map_data, &surface).unwrap();
                    let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
                    let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();

                    let targets = compile_height_targets(&graph, &guide, &config).unwrap();
                    let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
                    let layer =
                        solve_surface_heights(&graph, &guide, &targets, &hard, &config).unwrap();

                    validate_surface_height_layer(&layer, &graph, &guide, &hard, &config).unwrap();

                    let bits: Vec<u32> = layer.heights.iter().map(|h| h.to_bits()).collect();
                    if let Some(ref base) = baseline_bits {
                        assert_eq!(&bits, base);
                    } else {
                        baseline_bits = Some(bits);
                    }
                }
            }
        }
    }

    // ─── Canonical 144 — semantic surface height ─────────────────────────────

    /// Proof: 6 shapes × 3 profiles × 8 seeds with deterministic elevation ranks,
    /// region intents (Mountain/Plateau/Lake/River/None), and resolved cliff decorator.
    /// Verifies solver handles full semantic surface for all geometry shapes.
    #[test]
    fn canonical_144_semantic_surface_height_matrix() {
        let config = HeightSolverConfig::default();

        for (shape_name, mut map_template) in q::all_shapes() {
            // 1. Sort all tile coords deterministically
            let sorted_coords: Vec<HexCoord> = {
                let mut coords: Vec<_> = map_template.tiles.keys().copied().collect();
                coords.sort_by(|a, b| a.q.cmp(&b.q).then_with(|| a.r.cmp(&b.r)));
                coords
            };

            // 2. Assign deterministic elevation rank per tile index
            for (idx, coord) in sorted_coords.iter().enumerate() {
                if let Some(tile) = map_template.tiles.get_mut(coord) {
                    let t = (idx % 8) as f32 / 7.0;
                    tile.elevation = (0.10 + 0.70 * t).clamp(0.0, 1.0);
                }
            }

            // 3. Assign deterministic region semantics (all 4 M5 region types)
            for (idx, coord) in sorted_coords.iter().enumerate() {
                if let Some(tile) = map_template.tiles.get_mut(coord) {
                    tile.landscape_feature = match idx % 5 {
                        0 => LandscapeFeature::None,
                        1 => LandscapeFeature::Mountain,
                        2 => LandscapeFeature::Plateau,
                        3 => LandscapeFeature::Lake,
                        _ => LandscapeFeature::River,
                    };
                }
            }

            // 4. Derive resolved cliffs from strict scalar rank ordering (no directed cycles possible)
            let cliff_edges = derive_ranked_cliff_edges(&map_template, &sorted_coords);
            for (edge, data) in cliff_edges {
                map_template.edges.insert(edge, data);
            }

            for profile in q::all_profiles() {
                for seed_val in q::FAST_SEEDS {
                    let seed = WorldSeed::new(seed_val);
                    let face_top =
                        generate_hex_face_topology_with_profile(&map_template, seed, profile)
                            .unwrap();
                    let surface = generate_surface_topology(&face_top).unwrap();
                    let constraints = compile_height_constraints(&map_template, &surface).unwrap();
                    let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
                    let guide =
                        derive_legacy_height_guide(&map_template, &surface, &graph).unwrap();

                    let targets = compile_height_targets(&graph, &guide, &config)
                        .unwrap_or_else(|e| panic!("targets {shape_name} seed={seed_val}: {e:?}"));
                    let hard = compile_hard_constraints(&graph, &guide, &config)
                        .unwrap_or_else(|e| panic!("hard {shape_name} seed={seed_val}: {e:?}"));
                    let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config)
                        .unwrap_or_else(|e| panic!("solver {shape_name} seed={seed_val}: {e:?}"));
                    validate_surface_height_layer(&layer, &graph, &guide, &hard, &config)
                        .unwrap_or_else(|e| panic!("validate {shape_name} seed={seed_val}: {e:?}"));
                }
            }
        }
    }

    /// Deterministic cliff edges from elevation rank ordering.
    /// Visits each canonical logical edge exactly once via the coord == edge.a guard.
    /// Directed by strict rank: no cycles possible by construction.
    fn derive_ranked_cliff_edges(
        map: &MapData,
        sorted_coords: &[HexCoord],
    ) -> Vec<(EdgeCoord, EdgeData)> {
        let rank: HashMap<HexCoord, usize> = sorted_coords
            .iter()
            .enumerate()
            .map(|(i, &c)| (c, i))
            .collect();

        let mut edges = Vec::new();
        for &coord in sorted_coords {
            for neighbor in coord.neighbors() {
                if !map.tiles.contains_key(&neighbor) {
                    continue;
                }

                let edge = EdgeCoord::new(coord, neighbor);

                // Visit each canonical logical edge exactly once
                if coord != edge.a {
                    continue;
                }

                let rank_a = rank[&edge.a];
                let rank_b = rank[&edge.b];

                debug_assert_ne!(rank_a, rank_b, "elevation ranks should differ");

                let cliff_lower_side = if rank_a < rank_b {
                    CliffLowerSide::A
                } else {
                    CliffLowerSide::B
                };

                edges.push((
                    edge,
                    EdgeData {
                        edge_type: EdgeType::Cliff,
                        cliff_lower_side,
                    },
                ));
            }
        }
        edges
    }
}
