/// Geometry validation for hex face topology.
use crate::map::data::HEX_SIZE;
use crate::map::face_topology::types::{FaceId, HalfEdgeId, HexFaceTopologyError};
use bevy::prelude::Vec2;

pub const MIN_EDGE_LENGTH: f32 = 0.05 * HEX_SIZE;
const GEOMETRY_EPSILON: f32 = 1e-6;

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
        min_len = min_len.min(pts[i].distance(pts[(i + 1) % 6]));
    }
    min_len
}

fn orientation(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}

fn point_on_segment(a: Vec2, b: Vec2, point: Vec2) -> bool {
    point.x >= a.x.min(b.x) - GEOMETRY_EPSILON
        && point.x <= a.x.max(b.x) + GEOMETRY_EPSILON
        && point.y >= a.y.min(b.y) - GEOMETRY_EPSILON
        && point.y <= a.y.max(b.y) + GEOMETRY_EPSILON
}

/// Tests if two line segments intersect, including touching and collinear overlap.
#[must_use]
pub fn segments_intersect(s1a: Vec2, s1b: Vec2, s2a: Vec2, s2b: Vec2) -> bool {
    let o1 = orientation(s1a, s1b, s2a);
    let o2 = orientation(s1a, s1b, s2b);
    let o3 = orientation(s2a, s2b, s1a);
    let o4 = orientation(s2a, s2b, s1b);
    let proper = ((o1 > GEOMETRY_EPSILON && o2 < -GEOMETRY_EPSILON)
        || (o1 < -GEOMETRY_EPSILON && o2 > GEOMETRY_EPSILON))
        && ((o3 > GEOMETRY_EPSILON && o4 < -GEOMETRY_EPSILON)
            || (o3 < -GEOMETRY_EPSILON && o4 > GEOMETRY_EPSILON));
    proper
        || (o1.abs() <= GEOMETRY_EPSILON && point_on_segment(s1a, s1b, s2a))
        || (o2.abs() <= GEOMETRY_EPSILON && point_on_segment(s1a, s1b, s2b))
        || (o3.abs() <= GEOMETRY_EPSILON && point_on_segment(s2a, s2b, s1a))
        || (o4.abs() <= GEOMETRY_EPSILON && point_on_segment(s2a, s2b, s1b))
}

/// Validates a six-vertex polygon independently from the ECS world.
///
/// # Errors
/// Returns a geometry-specific `HexFaceTopologyError` when an invariant fails.
#[allow(clippy::needless_range_loop, clippy::similar_names)]
pub fn validate_face_geometry(
    pts: &[Vec2; 6],
    face_id: FaceId,
) -> Result<(), HexFaceTopologyError> {
    if pts
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite())
    {
        return Err(HexFaceTopologyError::ValidationFailed(format!(
            "Face {face_id:?} contains a non-finite vertex"
        )));
    }
    if signed_area(pts) <= 0.0 {
        return Err(HexFaceTopologyError::NonPositiveArea(face_id));
    }
    for i in 0..6 {
        if pts[i].distance(pts[(i + 1) % 6]) <= MIN_EDGE_LENGTH {
            return Err(HexFaceTopologyError::NearZeroEdge {
                face: face_id,
                edge: HalfEdgeId::new(i),
            });
        }
    }
    for i in 0..6 {
        if orientation(pts[i], pts[(i + 1) % 6], pts[(i + 2) % 6]) <= GEOMETRY_EPSILON {
            return Err(HexFaceTopologyError::NonConvexFace(face_id));
        }
    }
    for i in 0..6 {
        for j in (i + 2)..6 {
            if (i != 0 || j != 5)
                && segments_intersect(pts[i], pts[(i + 1) % 6], pts[j], pts[(j + 1) % 6])
            {
                return Err(HexFaceTopologyError::SelfIntersectingFace(face_id));
            }
        }
    }
    Ok(())
}
