// src/map/tools/landscape_edge_picker.rs
//! Editor-only derived index for warped landscape edge picking.

use crate::map::data::EdgeCoord;
use crate::map::face_topology::types::{FaceId, HalfEdgeId, HexFaceTopology, VertexId};
use bevy::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct EditableLandscapeEdge {
    pub logical_edge: EdgeCoord,

    pub face_a: FaceId,
    pub face_b: FaceId,

    pub half_edge_a: HalfEdgeId,
    pub half_edge_b: HalfEdgeId,

    pub vertices: [VertexId; 2],

    pub segment_start: Vec2,
    pub segment_end: Vec2,

    pub center_a: Vec2,
    pub center_b: Vec2,
}

#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct LandscapeEdgePickIndex {
    pub edges: Vec<EditableLandscapeEdge>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgePickIndexError {
    InvalidFace(FaceId),
    InvalidHalfEdge(HalfEdgeId),
    InvalidVertex(VertexId),
}

/// Builds the `LandscapeEdgePickIndex` from the authoritative `HexFaceTopology`.
///
/// # Errors
/// Returns `EdgePickIndexError` if the topology contains invalid face, half-edge, or vertex references.
pub fn build_landscape_edge_pick_index(
    topology: &HexFaceTopology,
) -> Result<LandscapeEdgePickIndex, EdgePickIndexError> {
    let mut edges = Vec::new();
    let mut visited_pairs = std::collections::HashSet::new();

    for (he_idx, he) in topology.half_edges.iter().enumerate() {
        let Some(twin_id) = he.twin else {
            continue;
        };
        let he_id = HalfEdgeId::new(he_idx);

        let pair_key = if he_id.index() < twin_id.index() {
            (he_id, twin_id)
        } else {
            (twin_id, he_id)
        };
        if !visited_pairs.insert(pair_key) {
            continue;
        }

        let twin = topology
            .half_edges
            .get(twin_id.index())
            .ok_or(EdgePickIndexError::InvalidHalfEdge(twin_id))?;

        let face_left_obj = topology
            .faces
            .get(he.incident_face.index())
            .ok_or(EdgePickIndexError::InvalidFace(he.incident_face))?;
        let face_right_obj = topology
            .faces
            .get(twin.incident_face.index())
            .ok_or(EdgePickIndexError::InvalidFace(twin.incident_face))?;

        let hex_left = face_left_obj.hex;
        let hex_right = face_right_obj.hex;

        let logical_edge = EdgeCoord::new(hex_left, hex_right);

        let (face_a, face_b, half_edge_a, half_edge_b) = if logical_edge.a == hex_left {
            (he.incident_face, twin.incident_face, he_id, twin_id)
        } else {
            (twin.incident_face, he.incident_face, twin_id, he_id)
        };

        let he_a_obj = topology
            .half_edges
            .get(half_edge_a.index())
            .ok_or(EdgePickIndexError::InvalidHalfEdge(half_edge_a))?;

        let v_origin = topology
            .vertices
            .get(he_a_obj.origin.index())
            .ok_or(EdgePickIndexError::InvalidVertex(he_a_obj.origin))?;
        let v_dest = topology
            .vertices
            .get(he_a_obj.destination.index())
            .ok_or(EdgePickIndexError::InvalidVertex(he_a_obj.destination))?;

        let segment_start = v_origin.position;
        let segment_end = v_dest.position;

        let compute_centroid = |face_id: FaceId| -> Result<Vec2, EdgePickIndexError> {
            let face_obj = topology
                .faces
                .get(face_id.index())
                .ok_or(EdgePickIndexError::InvalidFace(face_id))?;
            let mut sum = Vec2::ZERO;
            for &vid in &face_obj.vertices {
                let v = topology
                    .vertices
                    .get(vid.index())
                    .ok_or(EdgePickIndexError::InvalidVertex(vid))?;
                sum += v.position;
            }
            Ok(sum / 6.0)
        };

        let center_a = compute_centroid(face_a)?;
        let center_b = compute_centroid(face_b)?;

        edges.push(EditableLandscapeEdge {
            logical_edge,
            face_a,
            face_b,
            half_edge_a,
            half_edge_b,
            vertices: [he_a_obj.origin, he_a_obj.destination],
            segment_start,
            segment_end,
            center_a,
            center_b,
        });
    }

    edges.sort_by_key(|e| (e.logical_edge.a, e.logical_edge.b));

    Ok(LandscapeEdgePickIndex { edges })
}

pub fn rebuild_landscape_edge_pick_index(
    face_topology: Res<HexFaceTopology>,
    mut pick_index: Option<ResMut<LandscapeEdgePickIndex>>,
) {
    let Some(ref mut pick_index) = pick_index else {
        return;
    };
    if !face_topology.is_changed() && !pick_index.is_added() {
        return;
    }

    match build_landscape_edge_pick_index(&face_topology) {
        Ok(index) => {
            **pick_index = index;
        }
        Err(err) => {
            bevy::log::tracing::event!(
                bevy::log::tracing::Level::ERROR,
                error = ?err,
                "Failed to build LandscapeEdgePickIndex"
            );
            **pick_index = LandscapeEdgePickIndex::default();
        }
    }
}
