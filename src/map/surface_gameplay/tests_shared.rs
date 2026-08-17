// src/map/surface_gameplay/tests_shared.rs
//! Shared test fixtures for the surface_gameplay module: minimal 2-hex
//! worlds (plain and cliff-split) built from `SurfaceTopology` + bake.

#[cfg(test)]
pub mod shared {
    use crate::map::height_graph::types::HeightNodeId;
    use crate::map::surface_topology::types::{
        SurfaceFace, SurfaceFaceId, SurfaceFaceSource, SurfaceHalfEdge, SurfaceHalfEdgeId,
        SurfaceTopology, SurfaceVertex, SurfaceVertexId, SurfaceVertexSource,
    };
    use crate::map::terrain_bake::types::{
        CliffWallEndpoint, CliffWallSegment, SurfaceTerrainBake, TerrainBakeFace, TerrainBakeVertex,
    };
    use crate::map::HexCoord;
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct SurfaceGameplaySharedTestsPlugin;

    impl Plugin for SurfaceGameplaySharedTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    pub struct TwoHex {
        pub surface: SurfaceTopology,
        pub bake: SurfaceTerrainBake,
    }

    impl TwoHex {
        /// Overrides the center-node heights with the cliff-split values.
        pub fn with_cliff_heights(self, h4: f32, h5: f32) -> TwoHex {
            let mut this = self;
            this.bake.vertices[4].normalized_height = h4;
            this.bake.vertices[5].normalized_height = h5;
            this
        }
    }

    /// Plain 2-hex world: shared edge vertices v0/v1 are corners; centers are
    /// unique: v2 = center of A, v3 = center of B.
    /// nodes: 0=v0(A face), 1=v1(A face), 2=v2(center A), 3=v3(center B).
    pub fn two_hex_plain(h0: f32, h1: f32, h2: f32, h3: f32) -> TwoHex {
        two_hex_base(h0, h1, h2, h3, false)
    }

    /// Cliff-split 2-hex world: shared edge has divergent nodes; centers are
    /// unique: v2 = center of A, v3 = center of B.
    /// nodes: 0=v0(A), 1=v0(B), 2=v1(A), 3=v1(B), 4=v2(center A), 5=v3(center B).
    pub fn two_hex_cliff(h0: f32, h1: f32, h2: f32, h3: f32, h4: f32, h5: f32) -> TwoHex {
        two_hex_base(h0, h1, h2, h3, true).with_cliff_heights(h4, h5)
    }

    fn two_hex_base(h0: f32, h1: f32, h2: f32, h3: f32, split: bool) -> TwoHex {
        let hex_a = HexCoord::new(0, 0);
        let hex_b = HexCoord::new(1, 0);
        let v0 = SurfaceVertexId::new(0);
        let v1 = SurfaceVertexId::new(1);
        let v2 = SurfaceVertexId::new(2);
        let v3 = SurfaceVertexId::new(3);
        let face_a = SurfaceFaceId::new(0);
        let face_b = SurfaceFaceId::new(1);
        let (he0, he1, he2, he3, he4, he5) = (
            SurfaceHalfEdgeId::new(0),
            SurfaceHalfEdgeId::new(1),
            SurfaceHalfEdgeId::new(2),
            SurfaceHalfEdgeId::new(3),
            SurfaceHalfEdgeId::new(4),
            SurfaceHalfEdgeId::new(5),
        );

        let mut surface = SurfaceTopology::default();
        surface.vertices = vec![
            SurfaceVertex {
                position: Vec2::new(0.0, 0.0),
                source: SurfaceVertexSource::HexCorner {
                    source_vertex: crate::map::face_topology::VertexId::new(0),
                },
            },
            SurfaceVertex {
                position: Vec2::new(1.0, 0.0),
                source: SurfaceVertexSource::HexCorner {
                    source_vertex: crate::map::face_topology::VertexId::new(1),
                },
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
        surface.half_edges = vec![
            SurfaceHalfEdge {
                origin: v0,
                destination: v1,
                next: he1,
                prev: he2,
                twin: Some(he3),
                incident_face: face_a,
            },
            SurfaceHalfEdge {
                origin: v1,
                destination: v2,
                next: he2,
                prev: he0,
                twin: None,
                incident_face: face_a,
            },
            SurfaceHalfEdge {
                origin: v2,
                destination: v0,
                next: he0,
                prev: he1,
                twin: None,
                incident_face: face_a,
            },
            SurfaceHalfEdge {
                origin: v1,
                destination: v0,
                next: he4,
                prev: he5,
                twin: Some(he0),
                incident_face: face_b,
            },
            SurfaceHalfEdge {
                origin: v0,
                destination: v3,
                next: he5,
                prev: he3,
                twin: None,
                incident_face: face_b,
            },
            SurfaceHalfEdge {
                origin: v3,
                destination: v1,
                next: he3,
                prev: he4,
                twin: None,
                incident_face: face_b,
            },
        ];
        surface.faces = vec![
            SurfaceFace {
                vertices: [v0, v1, v2],
                boundary: he0,
                owner_hex: hex_a,
                source: SurfaceFaceSource {
                    sector: 0,
                    triangle: 0,
                },
            },
            SurfaceFace {
                vertices: [v1, v0, v3],
                boundary: he3,
                owner_hex: hex_b,
                source: SurfaceFaceSource {
                    sector: 0,
                    triangle: 0,
                },
            },
        ];

        let nodes: Vec<HeightNodeId> = (0..6).map(HeightNodeId::new).collect();
        let mut bake = SurfaceTerrainBake::default();
        if split {
            bake.vertices = vec![
                TerrainBakeVertex {
                    surface_vertex: v0,
                    height_node: nodes[0],
                    position_xz: Vec2::new(0.0, 0.0),
                    normalized_height: h0,
                    owner_hexes: vec![hex_a],
                },
                TerrainBakeVertex {
                    surface_vertex: v0,
                    height_node: nodes[1],
                    position_xz: Vec2::new(0.0, 0.0),
                    normalized_height: h1,
                    owner_hexes: vec![hex_b],
                },
                TerrainBakeVertex {
                    surface_vertex: v1,
                    height_node: nodes[2],
                    position_xz: Vec2::new(1.0, 0.0),
                    normalized_height: h2,
                    owner_hexes: vec![hex_a],
                },
                TerrainBakeVertex {
                    surface_vertex: v1,
                    height_node: nodes[3],
                    position_xz: Vec2::new(1.0, 0.0),
                    normalized_height: h3,
                    owner_hexes: vec![hex_b],
                },
                TerrainBakeVertex {
                    surface_vertex: v2,
                    height_node: nodes[4],
                    position_xz: Vec2::new(0.0, 1.0),
                    normalized_height: 0.5,
                    owner_hexes: vec![hex_a],
                },
                TerrainBakeVertex {
                    surface_vertex: v3,
                    height_node: nodes[5],
                    position_xz: Vec2::new(1.0, 1.0),
                    normalized_height: 0.5,
                    owner_hexes: vec![hex_b],
                },
            ];
            bake.faces = vec![
                TerrainBakeFace {
                    surface_face: face_a,
                    nodes: [nodes[0], nodes[2], nodes[4]],
                    owner_hex: hex_a,
                },
                TerrainBakeFace {
                    surface_face: face_b,
                    nodes: [nodes[3], nodes[1], nodes[5]],
                    owner_hex: hex_b,
                },
            ];
            bake.cliff_walls = vec![CliffWallSegment {
                primary_half_edge: he0,
                twin_half_edge: he3,
                primary_face: face_a,
                twin_face: face_b,
                endpoints: [
                    CliffWallEndpoint {
                        surface_vertex: v0,
                        primary_node: nodes[0],
                        twin_node: nodes[1],
                    },
                    CliffWallEndpoint {
                        surface_vertex: v1,
                        primary_node: nodes[2],
                        twin_node: nodes[3],
                    },
                ],
            }];
        } else {
            bake.vertices = vec![
                TerrainBakeVertex {
                    surface_vertex: v0,
                    height_node: nodes[0],
                    position_xz: Vec2::new(0.0, 0.0),
                    normalized_height: h0,
                    owner_hexes: vec![hex_a],
                },
                TerrainBakeVertex {
                    surface_vertex: v1,
                    height_node: nodes[1],
                    position_xz: Vec2::new(1.0, 0.0),
                    normalized_height: h1,
                    owner_hexes: vec![hex_b],
                },
                TerrainBakeVertex {
                    surface_vertex: v2,
                    height_node: nodes[2],
                    position_xz: Vec2::new(0.0, 1.0),
                    normalized_height: h2,
                    owner_hexes: vec![hex_a],
                },
                TerrainBakeVertex {
                    surface_vertex: v3,
                    height_node: nodes[3],
                    position_xz: Vec2::new(1.0, 1.0),
                    normalized_height: h3,
                    owner_hexes: vec![hex_b],
                },
            ];
            bake.faces = vec![
                TerrainBakeFace {
                    surface_face: face_a,
                    nodes: [nodes[0], nodes[1], nodes[2]],
                    owner_hex: hex_a,
                },
                TerrainBakeFace {
                    surface_face: face_b,
                    nodes: [nodes[1], nodes[0], nodes[3]],
                    owner_hex: hex_b,
                },
            ];
        }

        TwoHex { surface, bake }
    }
}
