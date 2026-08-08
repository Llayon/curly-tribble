// src/map/tools/cliff_edit.rs
//! Warped edge cliff picking and landscape editing.

use crate::map::data::EdgeCoord;
use crate::map::face_topology::types::VertexId;
use crate::map::tools::landscape_edge_picker::LandscapeEdgePickIndex;
use bevy::prelude::*;

pub const CLIFF_PICK_RADIUS_RATIO: f32 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalEdgeSide {
    A,
    B,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LandscapeEdgeHit {
    pub logical_edge: EdgeCoord,
    pub side: Option<LogicalEdgeSide>,
    pub vertices: [VertexId; 2],
    pub distance_squared: f32,
}

#[must_use]
pub fn distance_sq_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len_sq = ab.length_squared();
    if len_sq < 1e-8 {
        return p.distance_squared(a);
    }
    let t = ((p - a).dot(ab) / len_sq).clamp(0.0, 1.0);
    let projection = a + t * ab;
    p.distance_squared(projection)
}

#[must_use]
pub fn cross_2d(v: Vec2, w: Vec2) -> f32 {
    v.x * w.y - v.y * w.x
}

#[must_use]
pub fn classify_side(
    cursor_xz: Vec2,
    segment_start: Vec2,
    segment_end: Vec2,
    center_a: Vec2,
    center_b: Vec2,
) -> Option<LogicalEdgeSide> {
    let edge_vec = segment_end - segment_start;
    let sign_cursor = cross_2d(edge_vec, cursor_xz - segment_start);
    let sign_a = cross_2d(edge_vec, center_a - segment_start);
    let sign_b = cross_2d(edge_vec, center_b - segment_start);

    if sign_cursor.abs() < 1e-4 {
        None
    } else if (sign_cursor > 0.0 && sign_a > 0.0) || (sign_cursor < 0.0 && sign_a < 0.0) {
        Some(LogicalEdgeSide::A)
    } else if (sign_cursor > 0.0 && sign_b > 0.0) || (sign_cursor < 0.0 && sign_b < 0.0) {
        Some(LogicalEdgeSide::B)
    } else {
        None
    }
}

#[must_use]
pub fn pick_landscape_edge(
    cursor_xz: Vec2,
    pick_index: &LandscapeEdgePickIndex,
) -> Option<LandscapeEdgeHit> {
    let mut hits = Vec::new();

    for edge in &pick_index.edges {
        let dist_sq = distance_sq_to_segment(cursor_xz, edge.segment_start, edge.segment_end);
        let edge_len = (edge.segment_end - edge.segment_start).length();
        let max_dist = edge_len * CLIFF_PICK_RADIUS_RATIO;
        let max_dist_sq = max_dist * max_dist;

        if dist_sq <= max_dist_sq {
            let side = classify_side(
                cursor_xz,
                edge.segment_start,
                edge.segment_end,
                edge.center_a,
                edge.center_b,
            );

            hits.push(LandscapeEdgeHit {
                logical_edge: edge.logical_edge,
                side,
                vertices: edge.vertices,
                distance_squared: dist_sq,
            });
        }
    }

    hits.sort_by(|h1, h2| {
        h1.distance_squared
            .total_cmp(&h2.distance_squared)
            .then_with(|| {
                (h1.logical_edge.a, h1.logical_edge.b)
                    .cmp(&(h2.logical_edge.a, h2.logical_edge.b))
            })
    });

    hits.into_iter().next()
}
