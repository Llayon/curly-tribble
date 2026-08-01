// src/map/face_topology/validation.rs
/// Geometry and topology validation for hex face topology.
use crate::map::data::{MapData, HEX_SIZE};
use crate::map::face_topology::types::{FaceId, HalfEdgeId, HexFaceTopology, HexFaceTopologyError};
use bevy::prelude::*;
use std::collections::HashSet;

pub struct ValidationPlugin;
impl Plugin for ValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Computes the signed area of a 6-vertex 2D polygon using the Shoelace formula.
#[must_use]
pub fn signed_area(pts: &[Vec2; 6]) -> f32 {
    let mut area = 0.0;
    for i in 0..6 {
        let j = (i + 1) % 6;
        area += pts[i].x * pts[j].y - pts[j].x * pts[i].y;
    }
    0.5 * area
}

/// Computes minimum edge length of a 6-vertex 2D polygon.
#[must_use]
pub fn min_edge_length(pts: &[Vec2; 6]) -> f32 {
    let mut min_len = f32::INFINITY;
    for i in 0..6 {
        let j = (i + 1) % 6;
        let len = pts[i].distance(pts[j]);
        if len < min_len {
            min_len = len;
        }
    }
    min_len
}

/// Tests if line segment (s1a, s1b) intersects line segment (s2a, s2b).
#[must_use]
pub fn segments_intersect(s1a: Vec2, s1b: Vec2, s2a: Vec2, s2b: Vec2) -> bool {
    let ccw = |a: Vec2, b: Vec2, c: Vec2| -> f32 {
        (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
    };

    let ccw1 = ccw(s1a, s1b, s2a);
    let ccw2 = ccw(s1a, s1b, s2b);
    let ccw3 = ccw(s2a, s2b, s1a);
    let ccw4 = ccw(s2a, s2b, s1b);

    (ccw1 * ccw2 < -1e-6) && (ccw3 * ccw4 < -1e-6)
}

/// Validates that a 6-vertex polygon is simple, strictly convex, counter-clockwise,
/// free of self-intersections and near-zero edges.
///
/// # Errors
/// Returns `HexFaceTopologyError` if any geometric constraint is violated.
#[allow(
    clippy::missing_errors_doc,
    clippy::needless_range_loop,
    clippy::similar_names
)]
pub fn validate_face_geometry(
    pts: &[Vec2; 6],
    face_id: FaceId,
) -> Result<(), HexFaceTopologyError> {
    let area = signed_area(pts);
    if area <= 0.0 {
        return Err(HexFaceTopologyError::NonPositiveArea(face_id));
    }

    let min_len = min_edge_length(pts);
    let min_edge_threshold = 0.05 * HEX_SIZE;
    if min_len < min_edge_threshold {
        return Err(HexFaceTopologyError::NearZeroEdge {
            face: face_id,
            edge: HalfEdgeId::new(0),
        });
    }

    // Check cross products of consecutive edges for counter-clockwise convexity
    for i in 0..6 {
        let v0 = pts[i];
        let v1 = pts[(i + 1) % 6];
        let v2 = pts[(i + 2) % 6];
        let edge_a = v1 - v0;
        let edge_b = v2 - v1;
        let cross = edge_a.x * edge_b.y - edge_a.y * edge_b.x;
        if cross <= 0.0 {
            return Err(HexFaceTopologyError::NonConvexFace(face_id));
        }
    }

    // Check segment intersections between non-adjacent edges
    for i in 0..6 {
        let seg_a_start = pts[i];
        let seg_a_end = pts[(i + 1) % 6];
        for j in (i + 2)..6 {
            if i == 0 && j == 5 {
                continue;
            }
            let seg_b_start = pts[j];
            let seg_b_end = pts[(j + 1) % 6];
            if segments_intersect(seg_a_start, seg_a_end, seg_b_start, seg_b_end) {
                return Err(HexFaceTopologyError::SelfIntersectingFace(face_id));
            }
        }
    }

    Ok(())
}

/// Performs a final complete topology validation on the generated `HexFaceTopology`.
///
/// # Errors
/// Returns `HexFaceTopologyError` detailing any topological or geometric invariant violation.
#[allow(clippy::too_many_lines, clippy::similar_names)]
pub fn validate_complete_topology(
    topology: &HexFaceTopology,
    map_data: &MapData,
) -> Result<(), HexFaceTopologyError> {
    // 1. Bijection: hex_to_face and face count match map_data
    if topology.faces.len() != map_data.tiles.len() {
        return Err(HexFaceTopologyError::ValidationFailed(format!(
            "Face count mismatch: topology has {}, map_data has {}",
            topology.faces.len(),
            map_data.tiles.len()
        )));
    }
    if topology.hex_to_face.len() != map_data.tiles.len() {
        return Err(HexFaceTopologyError::ValidationFailed(format!(
            "hex_to_face count mismatch: {} vs {}",
            topology.hex_to_face.len(),
            map_data.tiles.len()
        )));
    }

    for &coord in map_data.tiles.keys() {
        let Some(&f_id) = topology.hex_to_face.get(&coord) else {
            return Err(HexFaceTopologyError::ValidationFailed(format!(
                "Missing hex_to_face mapping for tile {coord:?}"
            )));
        };
        let face = &topology.faces[f_id.index()];
        if face.hex != coord {
            return Err(HexFaceTopologyError::ValidationFailed(format!(
                "Face hex mismatch: expected {coord:?}, got {:?}",
                face.hex
            )));
        }
    }

    let mut directed_edges: HashSet<(usize, usize)> = HashSet::new();

    // 2. Validate every face
    for (f_idx, face) in topology.faces.iter().enumerate() {
        let f_id = FaceId::new(f_idx);

        // 6 unique vertices
        let v_set: HashSet<_> = face.vertices.iter().copied().collect();
        if v_set.len() != 6 {
            return Err(HexFaceTopologyError::ValidationFailed(format!(
                "Face {f_idx} does not have 6 unique vertices"
            )));
        }

        // Geometry validation
        let mut pts = [Vec2::ZERO; 6];
        for (k, &v_id) in face.vertices.iter().enumerate() {
            if v_id.index() >= topology.vertices.len() {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Face {f_idx} references invalid VertexId {v_id:?}"
                )));
            }
            pts[k] = topology.vertices[v_id.index()].position;
        }
        validate_face_geometry(&pts, f_id)?;

        // 6-edge Next/Prev cycles
        let mut curr = face.boundary;
        let mut count = 0;
        for _ in 0..6 {
            if curr.index() >= topology.half_edges.len() {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Face {f_idx} boundary points to invalid HalfEdgeId {curr:?}"
                )));
            }
            let edge = &topology.half_edges[curr.index()];
            if edge.incident_face != f_id {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Edge {curr:?} incident_face mismatch for face {f_idx}"
                )));
            }
            let next_edge = &topology.half_edges[edge.next.index()];
            if next_edge.prev != curr {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Edge {curr:?} next/prev inconsistency in face {f_idx}"
                )));
            }

            let pair = (edge.origin.index(), edge.destination.index());
            if !directed_edges.insert(pair) {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Duplicate directed edge {pair:?} found"
                )));
            }

            curr = edge.next;
            count += 1;
        }

        if curr != face.boundary || count != 6 {
            return Err(HexFaceTopologyError::ValidationFailed(format!(
                "Face {f_idx} Next cycle did not close after 6 steps"
            )));
        }

        let mut curr_p = face.boundary;
        for _ in 0..6 {
            curr_p = topology.half_edges[curr_p.index()].prev;
        }
        if curr_p != face.boundary {
            return Err(HexFaceTopologyError::ValidationFailed(format!(
                "Face {f_idx} Prev cycle did not close after 6 steps"
            )));
        }
    }

    // 3. Twin edge validation
    for (e_idx, edge) in topology.half_edges.iter().enumerate() {
        let e_id = HalfEdgeId::new(e_idx);
        let face_a_hex = topology.faces[edge.incident_face.index()].hex;

        if let Some(twin_id) = edge.twin {
            if twin_id.index() >= topology.half_edges.len() {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Edge {e_idx} has invalid twin HalfEdgeId {twin_id:?}"
                )));
            }
            let twin = &topology.half_edges[twin_id.index()];
            if twin.twin != Some(e_id) {
                return Err(HexFaceTopologyError::InconsistentTwin {
                    edge: e_id,
                    twin: twin_id,
                });
            }
            if twin.origin != edge.destination || twin.destination != edge.origin {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Twin edge {twin_id:?} endpoints not reversed for edge {e_id:?}"
                )));
            }
            let face_b_hex = topology.faces[twin.incident_face.index()].hex;
            if !face_a_hex.neighbors().contains(&face_b_hex) {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Twin faces {face_a_hex:?} and {face_b_hex:?} are not logical HexCoord neighbors"
                )));
            }
        }
    }

    // 4. Edge relation invariant
    let expected_half_edges =
        topology.stats.paired_edge_count * 2 + topology.stats.border_edge_count;
    if expected_half_edges != topology.half_edges.len() {
        return Err(HexFaceTopologyError::ValidationFailed(format!(
            "Edge relation invariant broken: paired*2 ({}) + border ({}) != half_edges ({})",
            topology.stats.paired_edge_count * 2,
            topology.stats.border_edge_count,
            topology.half_edges.len()
        )));
    }

    Ok(())
}
