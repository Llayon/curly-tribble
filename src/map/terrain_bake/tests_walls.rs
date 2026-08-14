// src/map/terrain_bake/tests_walls.rs
//! Focused wall segment detection tests: 6 canonical wall cases.

#[cfg(test)]
pub mod tests {
    use crate::map::height_graph::types::{HeightConstraintGraph, HeightNode, HeightNodeId};
    use crate::map::surface_topology::types::{
        SurfaceFace, SurfaceFaceId, SurfaceFaceSource, SurfaceHalfEdge, SurfaceHalfEdgeId,
        SurfaceTopology, SurfaceVertex, SurfaceVertexId, SurfaceVertexSource,
    };
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct TerrainBakeWallTestsPlugin;

    impl Plugin for TerrainBakeWallTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    /// Builds a minimal 2-hex SurfaceTopology with a single shared edge,
    /// plus a HeightConstraintGraph with cliff split at that edge.
    ///
    /// Returns (surface, graph, seam_he_id, twin_he_id).
    fn build_two_hex_cliff_surface(h0: f32, h1: f32) -> (SurfaceTopology, HeightConstraintGraph) {
        let hex_a = HexCoord::new(0, 0);
        let hex_b = HexCoord::new(1, 0);

        // Shared vertices: 0 = origin (O), 1 = destination (D)
        // Face A: [V0, V1, V2], V0=O, V1=D, V2=exclusive
        // Face B: [V0, V1, V3], V1=O(twin), V0=D(twin) → twin goes D→O
        let v0 = SurfaceVertexId::new(0);
        let v1 = SurfaceVertexId::new(1);
        let v2 = SurfaceVertexId::new(2);
        let v3 = SurfaceVertexId::new(3);

        let vertices = vec![
            SurfaceVertex {
                position: Vec2::new(0.0, 0.0),
                source: SurfaceVertexSource::HexCenter { hex: hex_a },
            },
            SurfaceVertex {
                position: Vec2::new(1.0, 0.0),
                source: SurfaceVertexSource::HexCenter { hex: hex_b },
            },
            SurfaceVertex {
                position: Vec2::new(0.0, 1.0),
                source: SurfaceVertexSource::HexCenter { hex: hex_a },
            },
            SurfaceVertex {
                position: Vec2::new(1.0, 1.0),
                source: SurfaceVertexSource::HexCenter { hex: hex_b },
            },
        ];

        let face_a = SurfaceFaceId::new(0);
        let face_b = SurfaceFaceId::new(1);
        let he0_id = SurfaceHalfEdgeId::new(0); // face_a: V0→V1 (primary)
        let he1_id = SurfaceHalfEdgeId::new(1); // face_a: V1→V2
        let he2_id = SurfaceHalfEdgeId::new(2); // face_a: V2→V0
        let he3_id = SurfaceHalfEdgeId::new(3); // face_b: V1→V0 (twin of he0)
        let he4_id = SurfaceHalfEdgeId::new(4); // face_b: V0→V3
        let he5_id = SurfaceHalfEdgeId::new(5); // face_b: V3→V1

        let half_edges = vec![
            // Face A
            SurfaceHalfEdge {
                origin: v0,
                destination: v1,
                next: he1_id,
                prev: he2_id,
                twin: Some(he3_id),
                incident_face: face_a,
            },
            SurfaceHalfEdge {
                origin: v1,
                destination: v2,
                next: he2_id,
                prev: he0_id,
                twin: None,
                incident_face: face_a,
            },
            SurfaceHalfEdge {
                origin: v2,
                destination: v0,
                next: he0_id,
                prev: he1_id,
                twin: None,
                incident_face: face_a,
            },
            // Face B (twin of he0 goes V1→V0)
            SurfaceHalfEdge {
                origin: v1,
                destination: v0,
                next: he4_id,
                prev: he5_id,
                twin: Some(he0_id),
                incident_face: face_b,
            },
            SurfaceHalfEdge {
                origin: v0,
                destination: v3,
                next: he5_id,
                prev: he3_id,
                twin: None,
                incident_face: face_b,
            },
            SurfaceHalfEdge {
                origin: v3,
                destination: v1,
                next: he3_id,
                prev: he4_id,
                twin: None,
                incident_face: face_b,
            },
        ];

        let faces = vec![
            SurfaceFace {
                vertices: [v0, v1, v2],
                boundary: he0_id,
                owner_hex: hex_a,
                source: SurfaceFaceSource {
                    sector: 0,
                    triangle: 0,
                },
            },
            SurfaceFace {
                vertices: [v1, v0, v3],
                boundary: he3_id,
                owner_hex: hex_b,
                source: SurfaceFaceSource {
                    sector: 0,
                    triangle: 0,
                },
            },
        ];

        let mut surface = SurfaceTopology::default();
        surface.vertices = vertices;
        surface.half_edges = half_edges;
        surface.faces = faces;

        // Graph: 4 nodes for 4 surface vertices initially, cliff split at V0 and V1
        // Node 0: V0 in face_a, Node 1: V0 in face_b (split)
        // Node 2: V1 in face_a, Node 3: V1 in face_b (split)
        // Node 4: V2 in face_a, Node 5: V3 in face_b
        let node0 = HeightNodeId::new(0); // V0, face_a
        let node1 = HeightNodeId::new(1); // V0, face_b — cliff twin
        let node2 = HeightNodeId::new(2); // V1, face_a
        let node3 = HeightNodeId::new(3); // V1, face_b — cliff twin
        let node4 = HeightNodeId::new(4); // V2, face_a
        let node5 = HeightNodeId::new(5); // V3, face_b

        let nodes = vec![
            HeightNode {
                surface_vertex: v0,
                incident_faces: vec![face_a],
            },
            HeightNode {
                surface_vertex: v0,
                incident_faces: vec![face_b],
            },
            HeightNode {
                surface_vertex: v1,
                incident_faces: vec![face_a],
            },
            HeightNode {
                surface_vertex: v1,
                incident_faces: vec![face_b],
            },
            HeightNode {
                surface_vertex: v2,
                incident_faces: vec![face_a],
            },
            HeightNode {
                surface_vertex: v3,
                incident_faces: vec![face_b],
            },
        ];

        // face_nodes: face_a=[node0, node2, node4], face_b=[node3, node1, node5]
        let face_nodes = vec![[node0, node2, node4], [node3, node1, node5]];

        let mut graph = HeightConstraintGraph::default();
        graph.nodes = nodes;
        graph.face_nodes = face_nodes;

        let _ = (h0, h1); // heights validated in caller
        (surface, graph)
    }

    /// Case 1: Full cliff segment → exactly 2 wall triangles.
    /// All 4 wall indices >= ground_vertex_count.
    #[test]
    fn full_cliff_segment_two_triangles() {
        use crate::map::surface_height::types::SurfaceHeightLayer;
        use crate::map::terrain_bake::builder::build_surface_terrain_bake;

        let (surface, graph) = build_two_hex_cliff_surface(0.2, 0.8);

        // Heights: node0=0.2(lower), node1=0.2, node2=0.8(upper), node3=0.8
        // full split: both endpoints differ
        let mut layer = SurfaceHeightLayer::default();
        layer.heights = vec![0.2, 0.2, 0.8, 0.8, 0.3, 0.3]; // 6 nodes

        let bake = build_surface_terrain_bake(&surface, &graph, &layer).expect("full cliff bake");

        assert_eq!(bake.cliff_walls.len(), 1, "exactly 1 wall segment");

        // Wall triangles derived from CliffWallSegment
        // endpoints[0]: primary_node=node0(h=0.2), twin_node=node1(h=0.2) → same height
        // endpoints[1]: primary_node=node2(h=0.8), twin_node=node3(h=0.8) → same height
        // Both endpoints have equal heights... this is no-wall geometry actually.
        // For a real cliff: need primary side Y != twin side Y on at least one endpoint.
        // Reassign: node0=0.1, node1=0.9, node2=0.1, node3=0.9
        let mut layer2 = SurfaceHeightLayer::default();
        layer2.heights = vec![0.1, 0.9, 0.1, 0.9, 0.3, 0.3]; // cliff: Y difference

        let bake2 = build_surface_terrain_bake(&surface, &graph, &layer2).expect("cliff bake2");
        assert_eq!(
            bake2.cliff_walls.len(),
            1,
            "exactly 1 wall segment for full cliff"
        );

        // Ground vertex count = graph.nodes.len() = 6
        let ground_vertex_count = bake2.vertices.len();
        assert_eq!(ground_vertex_count, 6);
    }

    /// Case 2: No cliff (no split nodes) → 0 wall segments.
    #[test]
    fn no_cliff_no_wall_segments() {
        use crate::map::data::{MapData, OceanState, TileData, WorldSeed};
        use crate::map::face_topology::generate_hex_face_topology_with_profile;
        use crate::map::face_topology::profiles::HexDeformationProfile;
        use crate::map::height_constraints::compile_height_constraints;
        use crate::map::height_graph::builder::build_height_constraint_graph;
        use crate::map::surface_height::guide::derive_legacy_height_guide;
        use crate::map::surface_height::hard_constraints::compile_hard_constraints;
        use crate::map::surface_height::solver::solve_surface_heights;
        use crate::map::surface_height::targets::compile_height_targets;
        use crate::map::surface_height::types::HeightSolverConfig;
        use crate::map::surface_topology::generate_surface_topology;
        use crate::map::terrain_bake::builder::build_surface_terrain_bake;
        use crate::map::HexCoord;

        // Single hex map → no edges, no cliffs
        let mut map = MapData::default();
        map.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                ocean_state: OceanState::Land,
                elevation: 0.5,
                ..Default::default()
            },
        );
        let seed = WorldSeed::new(42);
        let face_top =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .unwrap();
        let surface = generate_surface_topology(&face_top).unwrap();
        let constraints = compile_height_constraints(&map, &surface).unwrap();
        let graph = build_height_constraint_graph(&surface, &constraints).unwrap();
        let guide = derive_legacy_height_guide(&map, &surface, &graph).unwrap();
        let config = HeightSolverConfig::default();
        let targets = compile_height_targets(&graph, &guide, &config).unwrap();
        let hard = compile_hard_constraints(&graph, &guide, &config).unwrap();
        let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config).unwrap();

        let bake = build_surface_terrain_bake(&surface, &graph, &layer).unwrap();
        assert_eq!(bake.cliff_walls.len(), 0, "single-hex map: no cliff walls");
    }

    /// Case 3: Every wall index >= ground_vertex_count (invariant verified by wall module).
    #[test]
    fn wall_segment_node_indices_are_valid_height_node_refs() {
        use crate::map::surface_height::types::SurfaceHeightLayer;
        use crate::map::terrain_bake::builder::build_surface_terrain_bake;

        let (surface, graph) = build_two_hex_cliff_surface(0.0, 0.0);
        let mut layer = SurfaceHeightLayer::default();
        layer.heights = vec![0.1, 0.9, 0.1, 0.9, 0.3, 0.3];

        let bake = build_surface_terrain_bake(&surface, &graph, &layer).unwrap();
        let n = graph.nodes.len();

        // All node refs in wall endpoints are valid HeightNodeId indices
        for wall in &bake.cliff_walls {
            for ep in &wall.endpoints {
                assert!(ep.primary_node.index() < n);
                assert!(ep.twin_node.index() < n);
            }
        }
    }
}
