// src/map/face_topology/validation.rs
use crate::map::data::HEX_SIZE;
use crate::map::face_topology::types::{FaceId, HexFaceTopologyError};
use bevy::prelude::*;

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

/// Validates that a 6-vertex polygon is simple, strictly convex, counter-clockwise,
/// and free of near-zero edges.
///
/// # Errors
/// Returns `HexFaceTopologyError` if the face geometry is non-positive area, near-zero edge, or non-convex.
#[allow(clippy::missing_errors_doc, clippy::needless_range_loop)]
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
            edge: crate::map::face_topology::types::HalfEdgeId::new(0),
        });
    }

    // Check cross products of consecutive edges for counter-clockwise convexity
    for i in 0..6 {
        let p0 = pts[i];
        let p1 = pts[(i + 1) % 6];
        let p2 = pts[(i + 2) % 6];
        let v1 = p1 - p0;
        let v2 = p2 - p1;
        let cross = v1.x * v2.y - v1.y * v2.x;
        if cross <= 0.0 {
            return Err(HexFaceTopologyError::NonConvexFace(face_id));
        }
    }

    Ok(())
}
