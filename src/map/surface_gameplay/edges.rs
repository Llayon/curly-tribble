// src/map/surface_gameplay/edges.rs
//! Milestone M6 — per-edge metric derivation (twin half-edge pairs and cliff
//! walls). Extracted from `metrics.rs` to honour the 300-line file limit
//! (Guard #21).

use crate::map::data::EdgeCoord;
use crate::map::surface_gameplay::types::{EdgeSurfaceMetrics, SurfaceMetricsError};
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceTopology, SurfaceVertexId};
use crate::map::terrain_bake::types::SurfaceTerrainBake;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub struct SurfaceGameplayEdgesPlugin;

impl Plugin for SurfaceGameplayEdgesPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Collects per-edge metrics from reciprocal twin half-edge pairs and
/// `cliff_walls` (canonical pairs only: `he.index() < twin.index()`).
pub(super) fn collect_edge_metrics(
    surface: &SurfaceTopology,
    bake: &SurfaceTerrainBake,
) -> Result<(BTreeMap<EdgeCoord, EdgeSurfaceMetrics>, BTreeSet<EdgeCoord>), SurfaceMetricsError> {
    let mut edges: BTreeMap<EdgeCoord, EdgeSurfaceMetrics> = BTreeMap::new();
    let mut seam_edges: BTreeSet<EdgeCoord> = BTreeSet::new();

    for (he_idx, he) in surface.half_edges.iter().enumerate() {
        let Some(twin_id) = he.twin else {
            continue; // boundary edge — no inter-hex pair
        };
        if he_idx >= twin_id.index() {
            continue; // canonical pair only
        }

        let twin = surface
            .half_edges
            .get(twin_id.index())
            .ok_or(SurfaceMetricsError::InvalidSurfaceHalfEdge(twin_id))?;

        let face_id_primary = he.incident_face;
        let face_id_twin = twin.incident_face;
        let face_primary = surface
            .faces
            .get(face_id_primary.index())
            .ok_or(SurfaceMetricsError::InvalidSurfaceFace(face_id_primary))?;
        let face_twin = surface
            .faces
            .get(face_id_twin.index())
            .ok_or(SurfaceMetricsError::InvalidSurfaceFace(face_id_twin))?;

        if face_primary.owner_hex == face_twin.owner_hex {
            continue;
        }
        let edge = EdgeCoord::new(face_primary.owner_hex, face_twin.owner_hex);

        let jump = max_shared_vertex_jump(surface, bake, face_id_primary, face_id_twin)?;

        edges
            .entry(edge)
            .and_modify(|m| m.max_boundary_jump = m.max_boundary_jump.max(jump))
            .or_insert(EdgeSurfaceMetrics {
                max_boundary_jump: jump,
                height_seam: false,
            });
    }

    for wall in &bake.cliff_walls {
        let owner_a = surface
            .faces
            .get(wall.primary_face.index())
            .map(|f| f.owner_hex);
        let owner_b = surface
            .faces
            .get(wall.twin_face.index())
            .map(|f| f.owner_hex);
        if let (Some(a), Some(b)) = (owner_a, owner_b) {
            if a != b {
                seam_edges.insert(EdgeCoord::new(a, b));
            }
        }
    }

    Ok((edges, seam_edges))
}

/// Max absolute height delta across the two shared vertices of a face pair.
fn max_shared_vertex_jump(
    surface: &SurfaceTopology,
    bake: &SurfaceTerrainBake,
    face_id_primary: SurfaceFaceId,
    face_id_twin: SurfaceFaceId,
) -> Result<f32, SurfaceMetricsError> {
    let face_primary = surface
        .faces
        .get(face_id_primary.index())
        .ok_or(SurfaceMetricsError::InvalidSurfaceFace(face_id_primary))?;
    let face_twin = surface
        .faces
        .get(face_id_twin.index())
        .ok_or(SurfaceMetricsError::InvalidSurfaceFace(face_id_twin))?;

    let mut jump = 0.0f32;
    for vertex in face_primary.vertices.iter().copied() {
        if !face_twin.vertices.contains(&vertex) {
            continue;
        }
        let h_primary = node_height_at_vertex(bake, face_id_primary, face_primary, vertex)?;
        let h_twin = node_height_at_vertex(bake, face_id_twin, face_twin, vertex)?;
        jump = jump.max((h_primary - h_twin).abs());
    }
    Ok(jump)
}

fn node_height_at_vertex(
    bake: &SurfaceTerrainBake,
    face_id: SurfaceFaceId,
    face: &crate::map::surface_topology::types::SurfaceFace,
    vertex: SurfaceVertexId,
) -> Result<f32, SurfaceMetricsError> {
    let corner = face
        .vertices
        .iter()
        .position(|&v| v == vertex)
        .ok_or(SurfaceMetricsError::InvalidSurfaceFace(face_id))?;
    let baked_face = bake
        .faces
        .get(face_id.index())
        .ok_or(SurfaceMetricsError::InvalidSurfaceFace(face_id))?;
    let node = baked_face.nodes[corner];
    bake.vertices
        .get(node.index())
        .map(|v| v.normalized_height)
        .ok_or(SurfaceMetricsError::InvalidHeightNode(node))
}
