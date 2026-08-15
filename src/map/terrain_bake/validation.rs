// src/map/terrain_bake/validation.rs
//! Independent validation of a `SurfaceTerrainBake` against its source inputs.
//!
//! Every check is recomputed from `surface`, `graph`, and `heights` — never
//! read back from the bake itself (except where the bake *is* the truth, e.g.
//! vertex XZ derived from surface vertices).

use crate::map::height_graph::types::{HeightConstraintGraph, HeightNodeId};
use crate::map::surface_height::types::SurfaceHeightLayer;
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceTopology};
use crate::map::terrain_bake::types::{
    SurfaceTerrainBake, TerrainBakeStats, TerrainBakeValidationError,
};
use crate::map::terrain_bake::walls::{build_face_corner_map, detect_cliff_wall_segments};
use bevy::prelude::*;

pub struct TerrainBakeValidationPlugin;

impl Plugin for TerrainBakeValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Validates `bake` against `surface`, `graph`, and `heights` independently.
///
/// # Errors
/// Returns `TerrainBakeValidationError` on first discrepancy.
pub fn validate_surface_terrain_bake(
    bake: &SurfaceTerrainBake,
    surface: &SurfaceTopology,
    graph: &HeightConstraintGraph,
    heights: &SurfaceHeightLayer,
) -> Result<(), TerrainBakeValidationError> {
    // Empty is valid (empty contract)
    if graph.nodes.is_empty() && bake.vertices.is_empty() && bake.faces.is_empty() {
        return Ok(());
    }

    // ── Structural length checks (independent of bake internals) ────────────
    check_structural_lengths(bake, surface, graph, heights)?;

    // ── Per-vertex checks ────────────────────────────────────────────────────
    for (idx, v) in bake.vertices.iter().enumerate() {
        validate_bake_vertex(v, surface, graph, heights, idx)?;
    }

    // ── Per-face checks ──────────────────────────────────────────────────────
    for (face_idx, bake_face) in bake.faces.iter().enumerate() {
        let face_id = SurfaceFaceId::new(face_idx);
        let expected_nodes = graph.face_nodes[face_idx];
        if bake_face.nodes != expected_nodes {
            return Err(TerrainBakeValidationError::FaceNodesMismatch { face: face_id });
        }
        let surface_face = surface
            .faces
            .get(face_idx)
            .ok_or(TerrainBakeValidationError::FaceOwnerMismatch { face: face_id })?;
        if bake_face.owner_hex != surface_face.owner_hex {
            return Err(TerrainBakeValidationError::FaceOwnerMismatch { face: face_id });
        }
    }

    // ── Wall segments: EXACT set equality (not just count) ──────────────────
    check_wall_segments(bake, surface, graph)?;

    // ── Stats: split count recomputed independently, never read from bake ───
    let expected_stats = TerrainBakeStats {
        ground_vertex_count: bake.vertices.len(),
        ground_face_count: bake.faces.len(),
        cliff_wall_segment_count: bake.cliff_walls.len(),
        split_surface_vertex_count: recompute_split_count(surface.vertices.len(), graph),
    };
    if bake.stats != expected_stats {
        return Err(TerrainBakeValidationError::StatsMismatch);
    }

    Ok(())
}

fn check_structural_lengths(
    bake: &SurfaceTerrainBake,
    surface: &SurfaceTopology,
    graph: &HeightConstraintGraph,
    heights: &SurfaceHeightLayer,
) -> Result<(), TerrainBakeValidationError> {
    if surface.faces.len() != graph.face_nodes.len() {
        return Err(TerrainBakeValidationError::FaceNodeCountMismatch {
            expected: surface.faces.len(),
            actual: graph.face_nodes.len(),
        });
    }
    if heights.heights.len() != graph.nodes.len() {
        return Err(TerrainBakeValidationError::HeightCountMismatch {
            expected: graph.nodes.len(),
            actual: heights.heights.len(),
        });
    }
    if bake.vertices.len() != graph.nodes.len() {
        return Err(TerrainBakeValidationError::VertexCountMismatch {
            expected: graph.nodes.len(),
            actual: bake.vertices.len(),
        });
    }
    if bake.faces.len() != surface.faces.len() {
        return Err(TerrainBakeValidationError::FaceCountMismatch {
            expected: surface.faces.len(),
            actual: bake.faces.len(),
        });
    }
    Ok(())
}

fn validate_bake_vertex(
    v: &crate::map::terrain_bake::types::TerrainBakeVertex,
    surface: &SurfaceTopology,
    graph: &HeightConstraintGraph,
    heights: &SurfaceHeightLayer,
    idx: usize,
) -> Result<(), TerrainBakeValidationError> {
    let node_id = v.height_node;

    // HeightNodeId index matches position in Vec
    if node_id.index() != idx {
        return Err(TerrainBakeValidationError::NodeIndexMismatch {
            vertex_idx: idx,
            expected: HeightNodeId::new(idx),
        });
    }

    // Surface vertex matches
    let node = &graph.nodes[idx];
    if v.surface_vertex != node.surface_vertex {
        return Err(TerrainBakeValidationError::VertexSurfaceVertexMismatch { node: node_id });
    }

    // XZ position bit-exact
    let src_vertex = surface
        .vertices
        .get(node.surface_vertex.index())
        .ok_or(TerrainBakeValidationError::PositionMismatch { node: node_id })?;
    if v.position_xz.x.to_bits() != src_vertex.position.x.to_bits()
        || v.position_xz.y.to_bits() != src_vertex.position.y.to_bits()
    {
        return Err(TerrainBakeValidationError::PositionMismatch { node: node_id });
    }

    // Height bit-exact and in-range
    let h = heights.heights[idx];
    if !(0.0..=1.0).contains(&h) {
        return Err(TerrainBakeValidationError::HeightOutOfRange {
            node: node_id,
            height: h,
        });
    }
    if v.normalized_height.to_bits() != h.to_bits() {
        return Err(TerrainBakeValidationError::HeightMismatch { node: node_id });
    }

    // Owner hexes: STRICT build — every incident face must resolve, no
    // silent skipping of invalid face references.
    let mut expected_hexes = Vec::with_capacity(node.incident_faces.len());
    for &fid in &node.incident_faces {
        let face = surface.faces.get(fid.index()).ok_or(
            TerrainBakeValidationError::InvalidIncidentFace {
                node: node_id,
                face: fid,
            },
        )?;
        expected_hexes.push(face.owner_hex);
    }
    expected_hexes.sort_by_key(|c| (c.q, c.r));
    expected_hexes.dedup();
    if v.owner_hexes != expected_hexes {
        return Err(TerrainBakeValidationError::OwnerHexesMismatch { node: node_id });
    }
    Ok(())
}

fn check_wall_segments(
    bake: &SurfaceTerrainBake,
    surface: &SurfaceTopology,
    graph: &HeightConstraintGraph,
) -> Result<(), TerrainBakeValidationError> {
    let face_corner_map = build_face_corner_map(graph);
    let expected_walls = detect_cliff_wall_segments(surface, &face_corner_map)
        .map_err(|_| TerrainBakeValidationError::WallDetectionFailed)?;
    if bake.cliff_walls != expected_walls {
        return Err(TerrainBakeValidationError::WallSegmentMismatch {
            expected: expected_walls.len(),
            actual: bake.cliff_walls.len(),
        });
    }

    // Wall node references must be in range
    for wall in &bake.cliff_walls {
        for ep in &wall.endpoints {
            if ep.primary_node.index() >= graph.nodes.len()
                || ep.twin_node.index() >= graph.nodes.len()
            {
                return Err(TerrainBakeValidationError::WallNodeOutOfRange {
                    node: ep.primary_node,
                });
            }
        }
    }
    Ok(())
}

/// Independently counts `SurfaceVertexId`s mapped to more than one `HeightNodeId`.
/// Mirrors the builder's semantics (count > 1 nodes per surface vertex = 1 split)
/// but recomputes from source data so a tampered `bake.stats` is always caught.
fn recompute_split_count(surface_vertex_count: usize, graph: &HeightConstraintGraph) -> usize {
    let mut per_sv_counts = vec![0usize; surface_vertex_count];
    for node in &graph.nodes {
        let idx = node.surface_vertex.index();
        if idx < per_sv_counts.len() {
            per_sv_counts[idx] += 1;
        }
    }
    per_sv_counts.into_iter().filter(|&c| c > 1).count()
}
