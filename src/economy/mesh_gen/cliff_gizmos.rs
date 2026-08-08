//! Authoritative cliff gizmo visualization using warped `HexFaceTopology` half-edges.

use crate::map::data::CliffLowerSide;
use crate::map::face_topology::edge_binding::{BoundCliffEdge, BoundCliffEdges, CliffBindingError};
use crate::map::face_topology::types::HexFaceTopology;
use bevy::prelude::*;

pub struct CliffGizmosPlugin;

impl Plugin for CliffGizmosPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, PartialEq)]
pub struct CliffGizmoGeometry {
    pub segment_start: Vec2,
    pub segment_end: Vec2,
    pub arrow_targets: Vec<Vec2>,
}

/// Computes pure warped 2D geometry for a bound cliff edge from authoritative `HexFaceTopology`.
///
/// # Errors
/// Returns `CliffBindingError` if half-edge, origin/destination vertices, or incident faces are missing from `HexFaceTopology`.
pub fn compute_cliff_gizmo_geometry(
    bound_edge: &BoundCliffEdge,
    topology: &HexFaceTopology,
) -> Result<CliffGizmoGeometry, CliffBindingError> {
    let he_a = topology
        .half_edges
        .get(bound_edge.half_edge_a.index())
        .ok_or(CliffBindingError::MissingAdjacency(bound_edge.logical_edge))?;

    let v_origin = topology
        .vertices
        .get(he_a.origin.index())
        .ok_or(CliffBindingError::MissingAdjacency(bound_edge.logical_edge))?;
    let v_dest = topology
        .vertices
        .get(he_a.destination.index())
        .ok_or(CliffBindingError::MissingAdjacency(bound_edge.logical_edge))?;

    let segment_start = v_origin.position;
    let segment_end = v_dest.position;

    let compute_face_center = |face_idx: usize| -> Result<Vec2, CliffBindingError> {
        let face = topology
            .faces
            .get(face_idx)
            .ok_or(CliffBindingError::MissingFaceA(bound_edge.logical_edge))?;
        let mut sum = Vec2::ZERO;
        for &vid in &face.vertices {
            let v = topology
                .vertices
                .get(vid.index())
                .ok_or(CliffBindingError::MissingAdjacency(bound_edge.logical_edge))?;
            sum += v.position;
        }
        Ok(sum / 6.0)
    };

    let center_a = compute_face_center(bound_edge.face_a.index())?;
    let center_b = compute_face_center(bound_edge.face_b.index())?;

    let arrow_targets = match bound_edge.lower_side {
        CliffLowerSide::Unresolved => vec![center_a, center_b],
        CliffLowerSide::A => vec![center_a],
        CliffLowerSide::B => vec![center_b],
    };

    Ok(CliffGizmoGeometry {
        segment_start,
        segment_end,
        arrow_targets,
    })
}

pub fn draw_cliffs_gizmos(
    mut gizmos: Gizmos,
    bound_cliff_edges: Res<BoundCliffEdges>,
    topology: Res<HexFaceTopology>,
    config: Res<crate::map::terrain_gen::TerrainConfig>,
) {
    if !config.cliff_layer.is_visible() {
        return;
    }
    let y = 0.1;

    for edge in &bound_cliff_edges.edges {
        if let Ok(geom) = compute_cliff_gizmo_geometry(edge, &topology) {
            let start = Vec3::new(geom.segment_start.x, y, geom.segment_start.y);
            let end = Vec3::new(geom.segment_end.x, y, geom.segment_end.y);
            gizmos.line(start, end, Color::WHITE);

            let midpoint_2d = (geom.segment_start + geom.segment_end) * 0.5;
            let midpoint = Vec3::new(midpoint_2d.x, y, midpoint_2d.y);

            let segment_dir_2d = (geom.segment_end - geom.segment_start).normalize_or_zero();
            let perp_2d = Vec2::new(-segment_dir_2d.y, segment_dir_2d.x);
            let perp = Vec3::new(perp_2d.x, 0.0, perp_2d.y);

            for &target_center_2d in &geom.arrow_targets {
                let dir_2d = (target_center_2d - midpoint_2d).normalize_or_zero();
                let dir = Vec3::new(dir_2d.x, 0.0, dir_2d.y);
                let arrow_base = midpoint + dir * 0.15;
                let arrow_tip = midpoint + dir * 0.35;
                gizmos.line(arrow_base, arrow_tip, Color::BLACK);

                let head_left = arrow_tip - dir * 0.1 + perp * 0.08;
                let head_right = arrow_tip - dir * 0.1 - perp * 0.08;
                gizmos.line(arrow_tip, head_left, Color::BLACK);
                gizmos.line(arrow_tip, head_right, Color::BLACK);
            }
        }
    }
}
