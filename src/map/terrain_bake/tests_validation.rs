// src/map/terrain_bake/tests_validation.rs
//! Negative proof: `validate_surface_terrain_bake` must reject tampered bakes —
//! exact wall-set equality, independently recomputed split count, strict
//! owner-hex resolution (no silent `filter_map` repair), and source length checks.

#[cfg(test)]
pub mod tests {
    use crate::map::height_graph::types::HeightNodeId;
    use crate::map::surface_height::types::SurfaceHeightLayer;
    use crate::map::terrain_bake::builder::build_surface_terrain_bake;
    use crate::map::terrain_bake::tests_walls::tests::build_two_hex_cliff_surface;
    use crate::map::terrain_bake::types::{SurfaceTerrainBake, TerrainBakeValidationError};
    use crate::map::terrain_bake::validation::validate_surface_terrain_bake;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct TerrainBakeValidationTestsPlugin;

    impl Plugin for TerrainBakeValidationTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// Builds a valid two-hex cliff bake plus its source inputs.
    fn valid_two_hex_bake() -> (SurfaceTerrainBake, SurfaceHeightLayer) {
        let (surface, graph) = build_two_hex_cliff_surface(0.0, 0.0);
        let mut layer = SurfaceHeightLayer::default();
        layer.heights = vec![0.1, 0.9, 0.1, 0.9, 0.3, 0.3];
        let bake =
            build_surface_terrain_bake(&surface, &graph, &layer).expect("valid two-hex cliff bake");
        (bake, layer)
    }

    /// Positive control: the validator accepts a correct bake.
    #[test]
    fn validator_accepts_two_hex_cliff_bake() {
        let (bake, _layer) = valid_two_hex_bake();
        let (surface, graph) = build_two_hex_cliff_surface(0.0, 0.0);
        let mut layer = SurfaceHeightLayer::default();
        layer.heights = vec![0.1, 0.9, 0.1, 0.9, 0.3, 0.3];

        let result = validate_surface_terrain_bake(&bake, &surface, &graph, &layer);
        assert_eq!(result, Ok(()), "validator must accept a correct bake");
    }

    /// Tampered stats.split_surface_vertex_count must be caught by an
    /// independently recomputed value — never read back from the bake itself.
    #[test]
    fn validator_rejects_tampered_split_count() {
        let (mut bake, _layer) = valid_two_hex_bake();
        bake.stats.split_surface_vertex_count = 999;

        let (surface, graph) = build_two_hex_cliff_surface(0.0, 0.0);
        let mut layer = SurfaceHeightLayer::default();
        layer.heights = vec![0.1, 0.9, 0.1, 0.9, 0.3, 0.3];

        let result = validate_surface_terrain_bake(&bake, &surface, &graph, &layer);
        assert_eq!(
            result,
            Err(TerrainBakeValidationError::StatsMismatch),
            "validator must reject tampered split count"
        );
    }

    /// Cleared owner_hexes must be rejected — no silent repair via filter_map.
    #[test]
    fn validator_rejects_cleared_owner_hexes() {
        let (mut bake, _layer) = valid_two_hex_bake();
        bake.vertices[0].owner_hexes.clear();

        let (surface, graph) = build_two_hex_cliff_surface(0.0, 0.0);
        let mut layer = SurfaceHeightLayer::default();
        layer.heights = vec![0.1, 0.9, 0.1, 0.9, 0.3, 0.3];

        let result = validate_surface_terrain_bake(&bake, &surface, &graph, &layer);
        assert_eq!(
            result,
            Err(TerrainBakeValidationError::OwnerHexesMismatch {
                node: HeightNodeId::new(0),
            }),
            "validator must reject cleared owner_hexes"
        );
    }

    /// Dropped wall segments must be caught by exact wall-set equality,
    /// not merely a count comparison.
    #[test]
    fn validator_rejects_dropped_wall_segment() {
        let (mut bake, _layer) = valid_two_hex_bake();
        assert!(
            !bake.cliff_walls.is_empty(),
            "fixture must have cliff walls"
        );
        bake.cliff_walls.clear();

        let (surface, graph) = build_two_hex_cliff_surface(0.0, 0.0);
        let mut layer = SurfaceHeightLayer::default();
        layer.heights = vec![0.1, 0.9, 0.1, 0.9, 0.3, 0.3];

        let result = validate_surface_terrain_bake(&bake, &surface, &graph, &layer);
        assert_eq!(
            result,
            Err(TerrainBakeValidationError::WallSegmentMismatch {
                expected: 1,
                actual: 0,
            }),
            "validator must reject dropped wall segments"
        );
    }

    /// A heights layer shorter than the graph must be rejected up front.
    #[test]
    fn validator_rejects_truncated_heights() {
        let (bake, _layer) = valid_two_hex_bake();
        let (surface, graph) = build_two_hex_cliff_surface(0.0, 0.0);
        let mut short_layer = SurfaceHeightLayer::default();
        short_layer.heights = vec![0.1, 0.9]; // only 2 of 6 nodes

        let result = validate_surface_terrain_bake(&bake, &surface, &graph, &short_layer);
        assert_eq!(
            result,
            Err(TerrainBakeValidationError::HeightCountMismatch {
                expected: 6,
                actual: 2,
            }),
            "validator must reject truncated heights layer"
        );
    }
}
