// src/map/terrain_bake/validation.rs
//! Independent validation of a `SurfaceTerrainBake` against its source inputs.

use crate::map::height_graph::types::HeightConstraintGraph;
use crate::map::surface_height::types::SurfaceHeightLayer;
use crate::map::surface_topology::types::SurfaceFaceId;
use crate::map::surface_topology::types::SurfaceTopology;
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
/// All checks are structurally independent of the builder's logic.
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

    // Vertex count
    if bake.vertices.len() != graph.nodes.len() {
        return Err(TerrainBakeValidationError::VertexCountMismatch {
            expected: graph.nodes.len(),
            actual: bake.vertices.len(),
        });
    }

    // Face count
    if bake.faces.len() != surface.faces.len() {
        return Err(TerrainBakeValidationError::FaceCountMismatch {
            expected: surface.faces.len(),
            actual: bake.faces.len(),
        });
    }

    // Per-vertex checks
    for (idx, v) in bake.vertices.iter().enumerate() {
        let node_id = v.height_node;

        // HeightNodeId index matches position in Vec
        if node_id.index() != idx {
            return Err(TerrainBakeValidationError::NodeIndexMismatch {
                vertex_idx: idx,
                expected: crate::map::height_graph::types::HeightNodeId::new(idx),
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

        // Owner hexes: build expected and compare
        let mut expected_hexes: Vec<_> = node
            .incident_faces
            .iter()
            .filter_map(|&fid| surface.faces.get(fid.index()).map(|f| f.owner_hex))
            .collect();
        expected_hexes.sort_by_key(|c| (c.q, c.r));
        expected_hexes.dedup();
        if v.owner_hexes != expected_hexes {
            return Err(TerrainBakeValidationError::OwnerHexesMismatch { node: node_id });
        }
    }

    // Per-face checks
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

    // Wall segments: re-detect and compare count
    let face_corner_map = build_face_corner_map(graph);
    let expected_walls = detect_cliff_wall_segments(surface, &face_corner_map)
        .map_err(|_| TerrainBakeValidationError::StatsMismatch)?;
    if bake.cliff_walls.len() != expected_walls.len() {
        return Err(TerrainBakeValidationError::StatsMismatch);
    }

    // Stats
    let expected_stats = TerrainBakeStats {
        ground_vertex_count: bake.vertices.len(),
        ground_face_count: bake.faces.len(),
        cliff_wall_segment_count: bake.cliff_walls.len(),
        split_surface_vertex_count: bake.stats.split_surface_vertex_count,
    };
    if bake.stats != expected_stats {
        return Err(TerrainBakeValidationError::StatsMismatch);
    }

    Ok(())
}
