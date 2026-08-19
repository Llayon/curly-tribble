// src/map/surface_gameplay/tests_matrix.rs
//! Canonical 144-case gameplay matrix: 6 shapes × 3 profiles × 8 seeds run
//! the full pipeline (topology → heights → bake → metrics → gameplay) and
//! assert cell/edge policy invariants, stats consistency, and an A* smoke
//! check over pure gameplay data (no `MapData` in pathfinding).

#[cfg(test)]
pub mod tests {
    use crate::map::data::{MapData, OceanState, TerrainType, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::height_constraints::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::navigation::{compute_astar_path, world_to_grid};
    use crate::map::surface_gameplay::compiler::compile_surface_gameplay;
    use crate::map::surface_gameplay::config::SurfaceGameplayConfig;
    use crate::map::surface_gameplay::metrics::derive_surface_metrics;
    use crate::map::surface_gameplay::types::{SurfaceGameplayMap, TraversalBlockReason};
    use crate::map::surface_gameplay::world::gameplay_center_world_pos;
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
    use std::collections::HashMap;

    #[allow(dead_code)]
    pub struct SurfaceGameplayMatrixTestsPlugin;

    impl Plugin for SurfaceGameplayMatrixTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// 6 shapes × 3 profiles × 8 seeds → authoritative gameplay compile with
    /// policy invariants and an A* smoke check on every case.
    #[test]
    fn canonical_144_gameplay_matrix() {
        let config = SurfaceGameplayConfig::default();
        let solver_config = HeightSolverConfig::default();
        let mut cases = 0;

        for (_shape, map_data) in q::all_shapes() {
            for profile in q::all_profiles() {
                for &seed_val in &q::FAST_SEEDS {
                    cases += 1;
                    let seed = WorldSeed::new(seed_val);
                    let face_top =
                        generate_hex_face_topology_with_profile(&map_data, seed, profile).unwrap();
                    let surface = generate_surface_topology(&face_top).unwrap();
                    let constraints = compile_height_constraints(&map_data, &surface).unwrap();
                    let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
                    let guide = derive_legacy_height_guide(&map_data, &surface, &graph).unwrap();
                    let targets = compile_height_targets(&graph, &guide, &solver_config).unwrap();
                    let hard = compile_hard_constraints(&graph, &guide, &solver_config).unwrap();
                    let layer =
                        solve_surface_heights(&graph, &guide, &targets, &hard, &solver_config)
                            .unwrap_or_else(|e| {
                                panic!("case {cases} seed={seed_val}: solve failed: {e:?}")
                            });
                    validate_surface_height_layer(&layer, &graph, &guide, &hard, &solver_config)
                        .unwrap();
                    let bake =
                        build_surface_terrain_bake(&surface, &graph, &layer).unwrap_or_else(|e| {
                            panic!("case {cases} seed={seed_val}: bake failed: {e:?}")
                        });
                    validate_surface_terrain_bake(&bake, &surface, &graph, &layer).unwrap();
                    let field = derive_surface_metrics(&surface, &bake).unwrap_or_else(|e| {
                        panic!("case {cases} seed={seed_val}: metrics failed: {e:?}")
                    });
                    let gameplay = compile_surface_gameplay(&field, &map_data, &config)
                        .unwrap_or_else(|e| {
                            panic!("case {cases} seed={seed_val}: compile failed: {e:?}")
                        });

                    assert_gameplay_invariants(&gameplay, &map_data, &config, cases, seed_val);
                    astar_smoke_check(&gameplay, cases, seed_val);
                }
            }
        }
    }

    /// Cell policy mirrors tile classification; buildable implies walkable;
    /// stats are consistent with the compiled maps.
    fn assert_gameplay_invariants(
        gameplay: &SurfaceGameplayMap,
        map_data: &MapData,
        config: &SurfaceGameplayConfig,
        cases: u32,
        seed_val: u32,
    ) {
        let msg = |what: &str| format!("case {cases} seed={seed_val}: {what}");

        assert_eq!(
            gameplay.cells.len(),
            map_data.tiles.len(),
            "{} tile-set must match exactly",
            msg("cell count")
        );

        for (hex, cell) in &gameplay.cells {
            let tile = &map_data.tiles[hex];
            assert_eq!(
                cell.walkable,
                tile.ocean_state == OceanState::Land,
                "{} walkable must mirror ocean state",
                msg("cell policy")
            );
            let expected_cost = match tile.terrain {
                TerrainType::Swamp => config.swamp_cost,
                TerrainType::Stony => config.stony_cost,
                _ => config.walk_base_cost,
            };
            assert_eq!(
                cell.movement_cost,
                expected_cost,
                "{} terrain cost mismatch",
                msg("cell policy")
            );
            assert!(
                cell.center_height.is_finite() && cell.relief.is_finite(),
                "{} finite solved geometry",
                msg("cell policy")
            );
            assert!(
                !cell.buildable || cell.walkable,
                "{} buildable implies walkable",
                msg("cell policy")
            );
        }

        for (edge, edge_metric) in &gameplay.edges {
            assert_eq!(
                edge_metric.block_reason.is_some(),
                !edge_metric.traversable,
                "{} block_reason must be present iff not traversable",
                msg("edge policy")
            );
            assert!(
                matches!(
                    edge_metric.block_reason,
                    None | Some(TraversalBlockReason::CliffSeam)
                        | Some(TraversalBlockReason::HeightStep)
                ),
                "{} unknown block reason",
                msg("edge policy")
            );
            assert!(
                edge_metric.center_height_delta.is_finite(),
                "{} finite height delta",
                msg("edge policy")
            );
        }

        assert_eq!(
            gameplay.stats.cell_count,
            gameplay.cells.len(),
            "{} stats cell_count",
            msg("stats")
        );
        assert_eq!(
            gameplay.stats.walkable_cell_count,
            gameplay.cells.values().filter(|c| c.walkable).count(),
            "{} stats walkable_cell_count",
            msg("stats")
        );
        assert_eq!(
            gameplay.stats.buildable_cell_count,
            gameplay.cells.values().filter(|c| c.buildable).count(),
            "{} stats buildable_cell_count",
            msg("stats")
        );
        assert_eq!(
            gameplay.stats.edge_count,
            gameplay.edges.len(),
            "{} stats edge_count",
            msg("stats")
        );
        assert_eq!(
            gameplay.stats.traversable_edge_count
                + gameplay.stats.cliff_seam_edge_count
                + gameplay.stats.height_step_edge_count,
            gameplay.edges.len(),
            "{} stats edge partition",
            msg("stats")
        );
    }

    /// A* over pure gameplay data: any pair of walkable neighbor hexes must
    /// produce a path whose waypoints all snap back to the gameplay layer.
    fn astar_smoke_check(gameplay: &SurfaceGameplayMap, cases: u32, seed_val: u32) {
        let msg = |what: &str| format!("case {cases} seed={seed_val}: {what}");

        let dynamic = HashMap::<IVec2, u8>::new();
        let Some((start_hex, target_hex)) = gameplay
            .cells
            .keys()
            .filter(|hex| {
                gameplay.cells.get(hex).is_some_and(|c| c.walkable)
                    && hex
                        .neighbors()
                        .iter()
                        .any(|n| gameplay.cells.get(n).is_some_and(|c| c.walkable))
            })
            .map(|hex| {
                let target = hex
                    .neighbors()
                    .iter()
                    .find(|n| gameplay.cells.get(n).is_some_and(|c| c.walkable))
                    .copied()
                    .unwrap();
                (*hex, target)
            })
            .next()
        else {
            return;
        };

        let start_pos = gameplay_center_world_pos(start_hex, gameplay);
        let target_pos = gameplay_center_world_pos(target_hex, gameplay);
        let path = compute_astar_path(gameplay, &dynamic, start_pos, target_pos, 0.1)
            .unwrap_or_else(|| panic!("{} walkable neighbors must yield a path", msg("astar")));

        assert!(
            path.len() >= 2,
            "{} path must contain at least both endpoints",
            msg("astar")
        );
        for &point in &path {
            let cell = world_to_grid(point);
            let hex = crate::map::HexCoord::new(cell.x, cell.y);
            assert!(
                gameplay.cells.get(&hex).is_some_and(|c| c.walkable),
                "{} every waypoint must land on a walkable cell",
                msg("astar")
            );
        }
        assert_eq!(
            world_to_grid(*path.last().unwrap()),
            world_to_grid(target_pos),
            "{} path must end on the target cell",
            msg("astar")
        );
    }
}
