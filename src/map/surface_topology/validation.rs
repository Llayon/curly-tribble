// src/map/surface_topology/validation.rs
//! Pure validation logic for `SurfaceTopology` invariants.

use crate::map::surface_topology::types::{
    SurfaceFaceId, SurfaceHalfEdgeId, SurfaceTopology, SurfaceTopologyError, SurfaceVertexId,
    SurfaceVertexSource,
};
use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTopologyValidationPlugin;

impl Plugin for SurfaceTopologyValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Validates all structural, half-edge, twin, and provenance invariants of `SurfaceTopology`.
///
/// # Errors
/// Returns `SurfaceTopologyError` if any invariant is violated.
pub fn validate_surface_topology(surface: &SurfaceTopology) -> Result<(), SurfaceTopologyError> {
    // 1. Validate vertices & provenance
    for (idx, vertex) in surface.vertices.iter().enumerate() {
        let v_id = SurfaceVertexId::new(idx);
        if !vertex.position.x.is_finite() || !vertex.position.y.is_finite() {
            return Err(SurfaceTopologyError::InvalidSurfaceVertex(v_id));
        }

        match &vertex.source {
            SurfaceVertexSource::HexCorner { .. }
            | SurfaceVertexSource::HexCenter { .. }
            | SurfaceVertexSource::HexRadialMidpoint { .. } => {}
            SurfaceVertexSource::HexEdgeMidpoint { source_a, source_b } => {
                if source_a.index() >= source_b.index() {
                    return Err(SurfaceTopologyError::InvalidProvenance { vertex: v_id });
                }
            }
        }
    }

    // 2. Validate faces
    for (idx, face) in surface.faces.iter().enumerate() {
        let f_id = SurfaceFaceId::new(idx);
        for &v_id in &face.vertices {
            if v_id.index() >= surface.vertices.len() {
                return Err(SurfaceTopologyError::InvalidSurfaceVertex(v_id));
            }
        }

        if face.boundary.index() >= surface.half_edges.len() {
            return Err(SurfaceTopologyError::InvalidFaceBoundary {
                face: f_id,
                edge: face.boundary,
            });
        }
    }

    // 3. Validate half-edges & cycles
    for (idx, he) in surface.half_edges.iter().enumerate() {
        let he_id = SurfaceHalfEdgeId::new(idx);

        if he.origin.index() >= surface.vertices.len() {
            return Err(SurfaceTopologyError::InvalidSurfaceVertex(he.origin));
        }
        if he.destination.index() >= surface.vertices.len() {
            return Err(SurfaceTopologyError::InvalidSurfaceVertex(he.destination));
        }
        if he.incident_face.index() >= surface.faces.len() {
            return Err(SurfaceTopologyError::InvalidSurfaceFace(he.incident_face));
        }
        if he.next.index() >= surface.half_edges.len() {
            return Err(SurfaceTopologyError::InvalidSurfaceHalfEdge(he.next));
        }
        if he.prev.index() >= surface.half_edges.len() {
            return Err(SurfaceTopologyError::InvalidSurfaceHalfEdge(he.prev));
        }

        let next_he = &surface.half_edges[he.next.index()];
        let prev_he = &surface.half_edges[he.prev.index()];

        if next_he.origin != he.destination {
            return Err(SurfaceTopologyError::InvalidHalfEdgeCycle {
                face: he.incident_face,
            });
        }
        if prev_he.destination != he.origin {
            return Err(SurfaceTopologyError::InvalidHalfEdgeCycle {
                face: he.incident_face,
            });
        }

        if let Some(twin_id) = he.twin {
            if twin_id.index() >= surface.half_edges.len() {
                return Err(SurfaceTopologyError::InvalidSurfaceHalfEdge(twin_id));
            }
            let twin_he = &surface.half_edges[twin_id.index()];

            if twin_he.twin != Some(he_id) {
                return Err(SurfaceTopologyError::TwinMismatch {
                    edge: he_id,
                    twin: twin_id,
                });
            }
            if twin_he.origin != he.destination || twin_he.destination != he.origin {
                return Err(SurfaceTopologyError::TwinOrientationMismatch {
                    edge: he_id,
                    twin: twin_id,
                });
            }
        }
    }

    Ok(())
}
