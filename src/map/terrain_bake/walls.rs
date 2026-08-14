// src/map/terrain_bake/walls.rs
//! Cliff wall segment detection from half-edge seams in `SurfaceTopology`.
//! Extracted from `builder.rs` to honour the 300-line file limit (Guard #21).

use crate::map::height_graph::types::{HeightConstraintGraph, HeightNodeId};
use crate::map::surface_topology::types::{
    SurfaceFaceId, SurfaceHalfEdgeId, SurfaceTopology, SurfaceVertexId,
};
use crate::map::terrain_bake::types::{CliffWallEndpoint, CliffWallSegment, TerrainBakeBuildError};
use bevy::prelude::*;

pub struct TerrainBakeWallsPlugin;

impl Plugin for TerrainBakeWallsPlugin {
    fn build(&self, _app: &mut App) {}
}

// ─── Face-corner-to-node map ─────────────────────────────────────────────────

/// Builds a flat `face_corner_map` where entry `[face_id * 3 + corner]` = `HeightNodeId`.
///
/// Contract: `graph.face_nodes.len() == surface.faces.len()`.
pub(super) fn build_face_corner_map(graph: &HeightConstraintGraph) -> Vec<HeightNodeId> {
    let mut map = Vec::with_capacity(graph.face_nodes.len() * 3);
    for &[a, b, c] in &graph.face_nodes {
        map.push(a);
        map.push(b);
        map.push(c);
    }
    map
}

/// Returns the node at `(face_id, corner)` from the flat map.
#[inline]
pub(super) fn face_corner_node(
    map: &[HeightNodeId],
    face_idx: usize,
    corner: usize,
) -> Option<HeightNodeId> {
    map.get(face_idx * 3 + corner).copied()
}

// ─── Corner lookup ────────────────────────────────────────────────────────────

/// Finds the corner index (0, 1, or 2) in `face.vertices` matching `vertex_id`.
pub(super) fn find_corner(
    surface: &SurfaceTopology,
    face_id: SurfaceFaceId,
    vertex_id: SurfaceVertexId,
) -> Option<usize> {
    let face = surface.faces.get(face_id.index())?;
    face.vertices.iter().position(|&v| v == vertex_id)
}

// ─── Wall detection ───────────────────────────────────────────────────────────

/// Detects cliff wall segments: pairs of adjacent surface faces where the
/// shared edge's height-domain nodes diverge (cliff split).
///
/// Only processes canonical pairs where `he.index() < twin.index()`.
///
/// # Errors
/// Returns `TerrainBakeBuildError` if any half-edge or face index is invalid.
pub(super) fn detect_cliff_wall_segments(
    surface: &SurfaceTopology,
    face_corner_map: &[HeightNodeId],
) -> Result<Vec<CliffWallSegment>, TerrainBakeBuildError> {
    let mut walls = Vec::new();

    for (he_idx, he) in surface.half_edges.iter().enumerate() {
        let Some(twin_id) = he.twin else {
            continue; // boundary edge — no wall candidate
        };

        // Process only canonical pairs (primary index < twin index)
        if he_idx >= twin_id.index() {
            continue;
        }

        let twin = surface
            .half_edges
            .get(twin_id.index())
            .ok_or(TerrainBakeBuildError::InvalidSurfaceHalfEdge(twin_id))?;

        // Validate reciprocal pair
        if twin.twin != Some(SurfaceHalfEdgeId::new(he_idx)) {
            return Err(TerrainBakeBuildError::InvalidSurfaceHalfEdge(twin_id));
        }

        let face_a = he.incident_face;
        let face_b = twin.incident_face;
        let origin = he.origin; // shared vertex: O
        let destination = he.destination; // shared vertex: D

        // Corners in face_a for O and D
        let corner_primary_origin = find_corner(surface, face_a, origin)
            .ok_or(TerrainBakeBuildError::InvalidSurfaceFace(face_a))?;
        let corner_primary_dest = find_corner(surface, face_a, destination)
            .ok_or(TerrainBakeBuildError::InvalidSurfaceFace(face_a))?;

        // Corners in face_b for O and D (twin goes D→O)
        let corner_twin_origin = find_corner(surface, face_b, origin)
            .ok_or(TerrainBakeBuildError::InvalidSurfaceFace(face_b))?;
        let corner_twin_dest = find_corner(surface, face_b, destination)
            .ok_or(TerrainBakeBuildError::InvalidSurfaceFace(face_b))?;

        let node_primary_origin =
            face_corner_node(face_corner_map, face_a.index(), corner_primary_origin)
                .ok_or(TerrainBakeBuildError::InvalidSurfaceFace(face_a))?;
        let node_primary_dest =
            face_corner_node(face_corner_map, face_a.index(), corner_primary_dest)
                .ok_or(TerrainBakeBuildError::InvalidSurfaceFace(face_a))?;
        let node_twin_origin =
            face_corner_node(face_corner_map, face_b.index(), corner_twin_origin)
                .ok_or(TerrainBakeBuildError::InvalidSurfaceFace(face_b))?;
        let node_twin_dest = face_corner_node(face_corner_map, face_b.index(), corner_twin_dest)
            .ok_or(TerrainBakeBuildError::InvalidSurfaceFace(face_b))?;

        // Cliff seam: height-domain split on at least one shared vertex
        if node_primary_origin == node_twin_origin && node_primary_dest == node_twin_dest {
            continue; // no seam
        }

        walls.push(CliffWallSegment {
            primary_half_edge: SurfaceHalfEdgeId::new(he_idx),
            twin_half_edge: twin_id,
            primary_face: face_a,
            twin_face: face_b,
            endpoints: [
                CliffWallEndpoint {
                    surface_vertex: origin,
                    primary_node: node_primary_origin,
                    twin_node: node_twin_origin,
                },
                CliffWallEndpoint {
                    surface_vertex: destination,
                    primary_node: node_primary_dest,
                    twin_node: node_twin_dest,
                },
            ],
        });
    }

    Ok(walls)
}
