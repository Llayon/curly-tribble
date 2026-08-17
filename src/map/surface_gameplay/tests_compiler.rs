// src/map/surface_gameplay/tests_compiler.rs
//! Unit tests for `compile_surface_gameplay`: exact tile-set, cell policy
//! (walkable / costs / buildable gates), edge policy (seam, height step),
//! and stats.

#[cfg(test)]
pub mod tests {
    use crate::map::data::{MapData, OceanState, TerrainType, TileData};
    use crate::map::surface_gameplay::compiler::compile_surface_gameplay;
    use crate::map::surface_gameplay::config::SurfaceGameplayConfig;
    use crate::map::surface_gameplay::tests_shared::shared::two_hex_cliff;
    use crate::map::surface_gameplay::tests_shared::shared::two_hex_plain;
    use crate::map::surface_gameplay::tests_shared::shared::TwoHex;
    use crate::map::surface_gameplay::types::{
        SurfaceGameplayCompileError, SurfaceMetricField, TraversalBlockReason,
    };
    use crate::map::HexCoord;
    use bevy::prelude::*;
    use std::collections::HashMap;

    #[allow(dead_code)]
    pub struct SurfaceGameplayCompilerTestsPlugin;

    impl Plugin for SurfaceGameplayCompilerTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    fn tile(terrain: TerrainType, ocean_state: OceanState) -> TileData {
        TileData {
            terrain,
            ocean_state,
            ..Default::default()
        }
    }

    fn map_two_hex(a: TileData, b: TileData) -> MapData {
        let mut tiles = HashMap::new();
        tiles.insert(HexCoord::new(0, 0), a);
        tiles.insert(HexCoord::new(1, 0), b);
        MapData {
            width: 2,
            height: 1,
            tiles,
            edges: HashMap::new(),
            validation_errors: Vec::new(),
        }
    }

    fn grass_map() -> MapData {
        map_two_hex(
            tile(TerrainType::Grass, OceanState::Land),
            tile(TerrainType::Grass, OceanState::Land),
        )
    }

    fn plain_field(h0: f32, h1: f32, h2: f32, h3: f32) -> SurfaceMetricField {
        let TwoHex { surface, bake } = two_hex_plain(h0, h1, h2, h3);
        crate::map::surface_gameplay::metrics::derive_surface_metrics(&surface, &bake)
            .expect("metrics must succeed")
    }

    fn compile(
        field: &SurfaceMetricField,
        map_data: &MapData,
        config: &SurfaceGameplayConfig,
    ) -> crate::map::surface_gameplay::types::SurfaceGameplayMap {
        compile_surface_gameplay(field, map_data, config)
            .expect("default config and matching tiles must compile")
    }

    // ─── Tile-set exactness ───────────────────────────────────────────────────

    #[test]
    fn metric_without_tile_rejected() {
        let mut field = plain_field(0.3, 0.3, 0.0, 0.0);
        let extra = field.cells[&HexCoord::new(1, 0)].clone();
        field.cells.insert(HexCoord::new(2, 0), extra);
        let map_data = grass_map();
        let err = compile_surface_gameplay(&field, &map_data, &SurfaceGameplayConfig::default())
            .expect_err("metric without tile must fail");
        assert_eq!(
            err,
            SurfaceGameplayCompileError::MetricWithoutTile(HexCoord::new(2, 0))
        );
    }

    #[test]
    fn tile_without_metric_rejected() {
        let mut field = plain_field(0.3, 0.3, 0.0, 0.0);
        field.cells.remove(&HexCoord::new(1, 0));
        let map_data = grass_map();
        let err = compile_surface_gameplay(&field, &map_data, &SurfaceGameplayConfig::default())
            .expect_err("missing metric must fail");
        assert_eq!(
            err,
            SurfaceGameplayCompileError::MissingMetricsForTile(HexCoord::new(1, 0))
        );
    }

    #[test]
    fn invalid_config_rejected() {
        let field = plain_field(0.3, 0.3, 0.0, 0.0);
        let map_data = grass_map();
        let mut config = SurfaceGameplayConfig::default();
        config.max_walk_step = 1.5;
        let err = compile_surface_gameplay(&field, &map_data, &config)
            .expect_err("invalid config must fail");
        assert!(matches!(err, SurfaceGameplayCompileError::InvalidConfig(_)));
    }

    // ─── Cell policy ──────────────────────────────────────────────────────────

    #[test]
    fn land_cells_are_walkable_with_base_cost() {
        let field = plain_field(0.3, 0.3, 0.0, 0.0);
        let map_data = grass_map();
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        let cell_a = &gameplay.cells[&HexCoord::new(0, 0)];
        let cell_b = &gameplay.cells[&HexCoord::new(1, 0)];
        assert!(cell_a.walkable && cell_b.walkable);
        assert_eq!(cell_a.movement_cost, 20);
        assert_eq!(cell_b.movement_cost, 20);
    }

    #[test]
    fn swamp_and_stony_costs_applied() {
        let field = plain_field(0.3, 0.3, 0.0, 0.0);
        let map_data = map_two_hex(
            tile(TerrainType::Swamp, OceanState::Land),
            tile(TerrainType::Stony, OceanState::Land),
        );
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        assert_eq!(gameplay.cells[&HexCoord::new(0, 0)].movement_cost, 50);
        assert_eq!(gameplay.cells[&HexCoord::new(1, 0)].movement_cost, 80);
    }

    #[test]
    fn ocean_cells_are_not_walkable() {
        let field = plain_field(0.3, 0.3, 0.0, 0.0);
        let map_data = map_two_hex(
            tile(TerrainType::Grass, OceanState::Land),
            tile(TerrainType::Grass, OceanState::Ocean),
        );
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        let cell_a = &gameplay.cells[&HexCoord::new(0, 0)];
        let cell_b = &gameplay.cells[&HexCoord::new(1, 0)];
        assert!(cell_a.walkable);
        assert!(!cell_b.walkable);
        assert!(cell_a.buildable);
        assert!(!cell_b.buildable);
    }

    #[test]
    fn swamp_blocks_buildings() {
        let field = plain_field(0.3, 0.3, 0.0, 0.0);
        let map_data = map_two_hex(
            tile(TerrainType::Swamp, OceanState::Land),
            tile(TerrainType::Grass, OceanState::Land),
        );
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        assert!(!gameplay.cells[&HexCoord::new(0, 0)].buildable);
        assert!(gameplay.cells[&HexCoord::new(1, 0)].buildable);
    }

    #[test]
    fn relief_gate_blocks_buildings() {
        // Hex A relief = |0.0 - 0.4| = 0.4 > 0.3; hex B relief = 0.0.
        let field = plain_field(0.0, 0.3, 0.4, 0.3);
        let map_data = grass_map();
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        assert!(!gameplay.cells[&HexCoord::new(0, 0)].buildable);
        assert!(gameplay.cells[&HexCoord::new(1, 0)].buildable);
    }

    #[test]
    fn neighbor_step_gate_blocks_buildings() {
        // Centers 0.0 vs 0.4: delta 0.4 > 0.3 blocks both cells.
        let field = plain_field(0.0, 0.0, 0.0, 0.4);
        let map_data = grass_map();
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        assert!(!gameplay.cells[&HexCoord::new(0, 0)].buildable);
        assert!(!gameplay.cells[&HexCoord::new(1, 0)].buildable);
    }

    #[test]
    fn flat_land_is_buildable() {
        let field = plain_field(0.3, 0.3, 0.0, 0.0);
        let map_data = grass_map();
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        assert!(gameplay.cells[&HexCoord::new(0, 0)].buildable);
        assert!(gameplay.cells[&HexCoord::new(1, 0)].buildable);
    }

    // ─── Edge policy ──────────────────────────────────────────────────────────

    #[test]
    fn edge_walkable_within_step() {
        let field = plain_field(0.0, 0.0, 0.0, 0.1);
        let map_data = grass_map();
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        let edge = &gameplay.edges
            [&crate::map::data::EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0))];
        assert!(edge.traversable);
        assert_eq!(edge.block_reason, None);
        assert_eq!(edge.center_height_delta, 0.1);
    }

    #[test]
    fn edge_equal_step_allowed() {
        // delta == max_walk_step (0.3) is permitted.
        let field = plain_field(0.0, 0.0, 0.0, 0.3);
        let map_data = grass_map();
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        let edge = &gameplay.edges
            [&crate::map::data::EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0))];
        assert!(edge.traversable);
        assert_eq!(edge.block_reason, None);
    }

    #[test]
    fn edge_steep_step_blocks_traversal() {
        let field = plain_field(0.0, 0.0, 0.0, 0.35);
        let map_data = grass_map();
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        let edge = &gameplay.edges
            [&crate::map::data::EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0))];
        assert!(!edge.traversable);
        assert_eq!(edge.block_reason, Some(TraversalBlockReason::HeightStep));
    }

    #[test]
    fn seam_edge_blocks_traversal_and_buildings() {
        let TwoHex { surface, bake } = two_hex_cliff(0.5, 0.5, 0.5, 0.5, 0.0, 0.3);
        let field = crate::map::surface_gameplay::metrics::derive_surface_metrics(&surface, &bake)
            .expect("metrics must succeed");
        assert_eq!(field.stats.seam_edge_count, 1);
        let map_data = grass_map();
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        let edge = &gameplay.edges
            [&crate::map::data::EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0))];
        assert!(!edge.traversable);
        assert_eq!(edge.block_reason, Some(TraversalBlockReason::CliffSeam));
        assert!(!gameplay.cells[&HexCoord::new(0, 0)].buildable);
        assert!(!gameplay.cells[&HexCoord::new(1, 0)].buildable);
    }

    // ─── Stats ────────────────────────────────────────────────────────────────

    #[test]
    fn stats_count_cells_edges_and_blocks() {
        // delta 0.35 > max_walk_step: edge blocks both cells from building too.
        let field = plain_field(0.0, 0.0, 0.0, 0.35);
        let map_data = map_two_hex(
            tile(TerrainType::Swamp, OceanState::Land),
            tile(TerrainType::Grass, OceanState::Land),
        );
        let gameplay = compile(&field, &map_data, &SurfaceGameplayConfig::default());

        assert_eq!(gameplay.stats.cell_count, 2);
        assert_eq!(gameplay.stats.walkable_cell_count, 2);
        assert_eq!(gameplay.stats.buildable_cell_count, 0);
        assert_eq!(gameplay.stats.edge_count, 1);
        assert_eq!(gameplay.stats.traversable_edge_count, 0);
        assert_eq!(gameplay.stats.height_step_edge_count, 1);
        assert_eq!(gameplay.stats.cliff_seam_edge_count, 0);
    }
}
