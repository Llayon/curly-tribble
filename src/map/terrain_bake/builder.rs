// src/map/terrain_bake/builder.rs
//! Pure builder for `SurfaceTerrainBake` from M5 height-domain data.
//! No dependencies on `MapData`, `MAX_HEIGHT`, `Mesh`, or `EditorPhase`.

use crate::map::height_graph::types::{HeightConstraintGraph, HeightNodeId};
use crate::map::surface_height::types::SurfaceHeightLayer;
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceTopology};
use crate::map::terrain_bake::types::{
    SurfaceTerrainBake, TerrainBakeBuildError, TerrainBakeFace, TerrainBakeStats, TerrainBakeVertex,
};
use crate::map::terrain_bake::walls::{build_face_corner_map, detect_cliff_wall_segments};
use crate::map::HexCoord;
use bevy::prelude::*;

pub struct TerrainBakeBuilderPlugin;

impl Plugin for TerrainBakeBuilderPlugin {
    fn build(&self, _app: &mut App) {}
}

// ─── Public builder ───────────────────────────────────────────────────────────

/// Builds the authoritative `SurfaceTerrainBake` from M5 height-domain outputs.
///
/// # Errors
/// Returns `TerrainBakeBuildError` on any structural inconsistency.
pub fn build_surface_terrain_bake(
    surface: &SurfaceTopology,
    graph: &HeightConstraintGraph,
    heights: &SurfaceHeightLayer,
) -> Result<SurfaceTerrainBake, TerrainBakeBuildError> {
    // ── Empty contract ────────────────────────────────────────────────────────
    let all_empty = surface.vertices.is_empty()
        && surface.faces.is_empty()
        && graph.nodes.is_empty()
        && heights.heights.is_empty();
    if all_empty {
        return Ok(SurfaceTerrainBake::default());
    }

    // ── Structural size checks ────────────────────────────────────────────────
    if surface.faces.len() != graph.face_nodes.len() {
        return Err(TerrainBakeBuildError::FaceNodeCountMismatch {
            expected: surface.faces.len(),
            actual: graph.face_nodes.len(),
        });
    }
    if graph.nodes.len() != heights.heights.len() {
        return Err(TerrainBakeBuildError::HeightNodeCountMismatch {
            expected: graph.nodes.len(),
            actual: heights.heights.len(),
        });
    }
    if !surface.vertices.is_empty()
        && (graph.nodes.is_empty() || graph.face_nodes.is_empty() || heights.heights.is_empty())
    {
        return Err(TerrainBakeBuildError::PartialEmptyInputs {
            surface_vertices: surface.vertices.len(),
            surface_faces: surface.faces.len(),
            graph_nodes: graph.nodes.len(),
            height_count: heights.heights.len(),
        });
    }

    // ── Vertex bake (one per HeightNodeId) ───────────────────────────────────
    let mut vertices = Vec::with_capacity(graph.nodes.len());

    for node_idx in 0..graph.nodes.len() {
        let node_id = HeightNodeId::new(node_idx);
        let node = &graph.nodes[node_idx];

        let src_vertex = surface.vertices.get(node.surface_vertex.index()).ok_or(
            TerrainBakeBuildError::InvalidSurfaceVertex(node.surface_vertex),
        )?;

        let h = *heights
            .heights
            .get(node_idx)
            .ok_or(TerrainBakeBuildError::InvalidHeightNode(node_id))?;

        if !h.is_finite() {
            return Err(TerrainBakeBuildError::NonFiniteHeight(node_id));
        }
        if !(0.0..=1.0).contains(&h) {
            return Err(TerrainBakeBuildError::HeightOutOfRange {
                node: node_id,
                height: h,
            });
        }

        // Build owner_hexes with typed face-corner contract proof
        let owner_hexes = build_owner_hexes(surface, graph, node_idx, node_id)?;

        vertices.push(TerrainBakeVertex {
            surface_vertex: node.surface_vertex,
            height_node: node_id,
            position_xz: src_vertex.position,
            normalized_height: h,
            owner_hexes,
        });
    }

    // ── Face bake (one per SurfaceFace) ──────────────────────────────────────
    let mut faces = Vec::with_capacity(surface.faces.len());

    for (face_idx, surface_face) in surface.faces.iter().enumerate() {
        let face_id = SurfaceFaceId::new(face_idx);
        let nodes = graph.face_nodes[face_idx]; // [HeightNodeId; 3]

        // Bounds-check each node
        for &nid in &nodes {
            if nid.index() >= graph.nodes.len() {
                return Err(TerrainBakeBuildError::InvalidHeightNode(nid));
            }
        }

        faces.push(TerrainBakeFace {
            surface_face: face_id,
            nodes,
            owner_hex: surface_face.owner_hex,
        });
    }

    // ── Wall detection ────────────────────────────────────────────────────────
    let face_corner_map = build_face_corner_map(graph);
    let cliff_walls = detect_cliff_wall_segments(surface, &face_corner_map)?;

    // ── Stats ─────────────────────────────────────────────────────────────────
    let split_surface_vertex_count = compute_split_count(surface.vertices.len(), graph);

    let stats = TerrainBakeStats {
        ground_vertex_count: vertices.len(),
        ground_face_count: faces.len(),
        cliff_wall_segment_count: cliff_walls.len(),
        split_surface_vertex_count,
    };

    Ok(SurfaceTerrainBake {
        vertices,
        faces,
        cliff_walls,
        stats,
    })
}

// ─── Private helpers ──────────────────────────────────────────────────────────

/// Collects `owner_hexes` for a single node with typed face-corner contract proof.
fn build_owner_hexes(
    surface: &SurfaceTopology,
    graph: &HeightConstraintGraph,
    node_idx: usize,
    node_id: HeightNodeId,
) -> Result<Vec<HexCoord>, TerrainBakeBuildError> {
    let node = &graph.nodes[node_idx];
    let mut owner_hexes = Vec::with_capacity(node.incident_faces.len());

    for &face_id in &node.incident_faces {
        let face = surface.faces.get(face_id.index()).ok_or(
            TerrainBakeBuildError::InvalidIncidentFace {
                node: node_id,
                face: face_id,
            },
        )?;

        // Prove: find the corner where surface face vertex == node.surface_vertex
        let corner = face
            .vertices
            .iter()
            .position(|&v| v == node.surface_vertex)
            .ok_or(TerrainBakeBuildError::IncidentFaceMissingNode {
                node: node_id,
                face: face_id,
            })?;

        // Prove: graph.face_nodes[face][corner] == node_id
        let mapped_node = graph.face_nodes.get(face_id.index()).ok_or(
            TerrainBakeBuildError::InvalidIncidentFace {
                node: node_id,
                face: face_id,
            },
        )?[corner];

        if mapped_node != node_id {
            return Err(TerrainBakeBuildError::IncidentFaceMissingNode {
                node: node_id,
                face: face_id,
            });
        }

        owner_hexes.push(face.owner_hex);
    }

    owner_hexes.sort_by_key(|c| (c.q, c.r));
    owner_hexes.dedup();
    Ok(owner_hexes)
}

/// Counts how many `SurfaceVertexId`s map to more than one `HeightNodeId` (cliff splits).
fn compute_split_count(surface_vertex_count: usize, graph: &HeightConstraintGraph) -> usize {
    let mut per_sv_counts = vec![0usize; surface_vertex_count];
    for node in &graph.nodes {
        let idx = node.surface_vertex.index();
        if idx < per_sv_counts.len() {
            per_sv_counts[idx] += 1;
        }
    }
    // count > 1: one SurfaceVertex split into N nodes counts as 1 split vertex
    per_sv_counts.into_iter().filter(|&c| c > 1).count()
}
