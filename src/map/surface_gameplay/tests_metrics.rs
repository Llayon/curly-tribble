// src/map/surface_gameplay/tests_metrics.rs
//! Unit tests for `derive_surface_metrics`: centers, relief, internal steps,
//! edge jumps, seams, empty/partial-empty contracts, and determinism.

#[cfg(test)]
pub mod tests {
    use crate::map::height_graph::types::HeightNodeId;
    use crate::map::surface_gameplay::metrics::derive_surface_metrics;
    use crate::map::surface_gameplay::tests_shared::shared::two_hex_cliff;
    use crate::map::surface_gameplay::tests_shared::shared::two_hex_plain;
    use crate::map::surface_gameplay::tests_shared::shared::TwoHex;
    use crate::map::surface_gameplay::types::SurfaceMetricsError;
    use crate::map::surface_topology::types::{
        SurfaceTopology, SurfaceVertex, SurfaceVertexId, SurfaceVertexSource,
    };
    use crate::map::terrain_bake::types::{SurfaceTerrainBake, TerrainBakeVertex};
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceGameplayMetricsTestsPlugin;

    impl Plugin for SurfaceGameplayMetricsTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    // ─── Empty / partial-empty contracts ──────────────────────────────────────

    #[test]
    fn empty_inputs_yield_default_field() {
        let field =
            derive_surface_metrics(&SurfaceTopology::default(), &SurfaceTerrainBake::default())
                .expect("empty inputs must succeed");
        assert!(field.cells.is_empty());
        assert!(field.edges.is_empty());
        assert_eq!(field.stats.cell_count, 0);
        assert_eq!(field.stats.edge_count, 0);
        assert_eq!(field.stats.seam_edge_count, 0);
    }

    #[test]
    fn partial_empty_inputs_rejected() {
        let mut surface = SurfaceTopology::default();
        surface.vertices.push(SurfaceVertex {
            position: Vec2::ZERO,
            source: SurfaceVertexSource::HexCenter {
                hex: HexCoord::new(0, 0),
            },
        });
        let err = derive_surface_metrics(&surface, &SurfaceTerrainBake::default())
            .expect_err("partial empty must fail");
        assert!(matches!(
            err,
            SurfaceMetricsError::PartialEmptyInputs { .. }
        ));
    }

    // ─── Centers ──────────────────────────────────────────────────────────────

    #[test]
    fn missing_hex_center_rejected() {
        let TwoHex { surface, bake } = two_hex_plain(0.4, 0.7, 0.3, 0.8);
        let mut surface = surface;
        // Drop the HexCenter provenance of v3 (center of hex_b)
        if let Some(v) = surface.vertices.get_mut(3) {
            v.source = SurfaceVertexSource::HexCorner {
                source_vertex: crate::map::face_topology::VertexId::new(0),
            };
        }
        let err = derive_surface_metrics(&surface, &bake).expect_err("missing center must fail");
        assert!(matches!(
            err,
            SurfaceMetricsError::MissingHexCenter(HexCoord { q: 1, r: 0 })
        ));
    }

    #[test]
    fn duplicate_hex_center_rejected() {
        let TwoHex { surface, bake } = two_hex_plain(0.4, 0.7, 0.3, 0.8);
        let mut surface = surface;
        // v1 is a corner; make it a second center of hex_a
        if let Some(v) = surface.vertices.get_mut(1) {
            v.source = SurfaceVertexSource::HexCenter {
                hex: HexCoord::new(0, 0),
            };
        }
        let err = derive_surface_metrics(&surface, &bake).expect_err("duplicate center must fail");
        assert!(matches!(err, SurfaceMetricsError::DuplicateHexCenter(_)));
    }

    #[test]
    fn cliff_split_center_is_ambiguous() {
        // Center vertex v2 (hex_a) maps to TWO height nodes: ambiguous center.
        let TwoHex { surface, bake } = two_hex_cliff(0.2, 0.8, 0.7, 0.7, 0.3, 0.9);
        let mut bake = bake;
        let v2 = SurfaceVertexId::new(2);
        bake.vertices.push(TerrainBakeVertex {
            surface_vertex: v2,
            height_node: HeightNodeId::new(6),
            position_xz: Vec2::new(0.0, 1.0),
            normalized_height: 0.5,
            owner_hexes: vec![HexCoord::new(0, 0)],
        });
        let err = derive_surface_metrics(&surface, &bake).expect_err("split center must fail");
        assert!(matches!(
            err,
            SurfaceMetricsError::AmbiguousCenterHeightNode(_)
        ));
    }

    // ─── Per-hex metrics ──────────────────────────────────────────────────────

    #[test]
    fn plain_two_hex_metrics_are_exact() {
        let TwoHex { surface, bake } = two_hex_plain(0.4, 0.7, 0.3, 0.8);
        let field = derive_surface_metrics(&surface, &bake).expect("valid input must succeed");

        let hex_a = HexCoord::new(0, 0);
        let hex_b = HexCoord::new(1, 0);

        let cell_a = &field.cells[&hex_a];
        assert_eq!(cell_a.center_xz, Vec2::new(0.0, 1.0));
        assert_eq!(cell_a.center_height, 0.3);
        assert!((cell_a.relief - 0.1).abs() < 1e-6);
        assert!((cell_a.max_internal_step - 0.4).abs() < 1e-6);

        let cell_b = &field.cells[&hex_b];
        assert_eq!(cell_b.center_xz, Vec2::new(1.0, 1.0));
        assert_eq!(cell_b.center_height, 0.8);
        assert!((cell_b.relief - 0.1).abs() < 1e-6);
        assert!((cell_b.max_internal_step - 0.4).abs() < 1e-6);

        assert_eq!(field.stats.cell_count, 2);
    }

    // ─── Edge metrics ─────────────────────────────────────────────────────────

    #[test]
    fn plain_edge_jump_is_zero_with_shared_nodes() {
        let TwoHex { surface, bake } = two_hex_plain(0.4, 0.7, 0.3, 0.8);
        let field = derive_surface_metrics(&surface, &bake).expect("valid input must succeed");

        let edge = crate::map::data::EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        let edge_metrics = &field.edges[&edge];
        assert!((edge_metrics.max_boundary_jump).abs() < 1e-6);
        assert!(!edge_metrics.height_seam);
        assert_eq!(field.stats.edge_count, 1);
        assert_eq!(field.stats.seam_edge_count, 0);
    }

    #[test]
    fn cliff_edge_is_seam_with_full_jump() {
        let TwoHex { surface, bake } = two_hex_cliff(0.2, 0.8, 0.7, 0.7, 0.3, 0.9);
        // V0 pair: |0.2 - 0.8| = 0.6; V1 pair: |0.7 - 0.7| = 0.0 → jump = 0.6
        let field = derive_surface_metrics(&surface, &bake).expect("valid input must succeed");

        let edge = crate::map::data::EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0));
        let edge_metrics = &field.edges[&edge];
        assert!((edge_metrics.max_boundary_jump - 0.6).abs() < 1e-6);
        assert!(edge_metrics.height_seam);
        assert_eq!(field.stats.seam_edge_count, 1);
    }

    // ─── Determinism ──────────────────────────────────────────────────────────

    #[test]
    fn metrics_are_bit_exact_deterministic() {
        let TwoHex { surface, bake } = two_hex_cliff(0.2, 0.8, 0.7, 0.7, 0.3, 0.9);
        let a = derive_surface_metrics(&surface, &bake).expect("first pass");
        let b = derive_surface_metrics(&surface, &bake).expect("second pass");
        assert_eq!(a, b);

        let TwoHex {
            surface: surface2,
            bake: bake2,
        } = two_hex_cliff(0.2, 0.8, 0.7, 0.7, 0.3, 0.9);
        let c = derive_surface_metrics(&surface2, &bake2).expect("rebuilt pass");
        assert_eq!(a, c);
    }
}
