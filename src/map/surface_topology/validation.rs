// src/map/surface_topology/validation.rs
//! Pure validation logic for `SurfaceTopology` invariants.

use crate::map::face_topology::HexFaceTopology;
use crate::map::surface_topology::types::{
    SurfaceFaceId, SurfaceHalfEdgeId, SurfaceTopology, SurfaceTopologyError, SurfaceVertexId,
};
use bevy::prelude::*;
use std::collections::HashSet;

#[allow(dead_code)]
pub struct SurfaceTopologyValidationPlugin;

impl Plugin for SurfaceTopologyValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

#[inline]
fn triangle_signed_area(p0: Vec2, p1: Vec2, p2: Vec2) -> f32 {
    0.5 * ((p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y))
}

/// Validates generic 2-manifold topological invariants of `SurfaceTopology`.
///
/// # Errors
/// Returns `SurfaceTopologyError` if any 2-manifold topological invariant is violated.
#[allow(clippy::too_many_lines, clippy::similar_names)]
pub fn validate_surface_topology(surface: &SurfaceTopology) -> Result<(), SurfaceTopologyError> {
    // 1. Validate vertices & positions
    for (idx, vertex) in surface.vertices.iter().enumerate() {
        let v_id = SurfaceVertexId::new(idx);
        if !vertex.position.x.is_finite() || !vertex.position.y.is_finite() {
            return Err(SurfaceTopologyError::InvalidSurfaceVertex(v_id));
        }
    }

    // 2. Validate faces & 3-cycles
    for (idx, face) in surface.faces.iter().enumerate() {
        let f_id = SurfaceFaceId::new(idx);
        let [v0, v1, v2] = face.vertices;

        if v0 == v1 || v1 == v2 || v2 == v0 {
            return Err(SurfaceTopologyError::DegenerateTriangle {
                hex: face.owner_hex,
                sector: face.source.sector,
                triangle: face.source.triangle,
            });
        }
        if v0.index() >= surface.vertices.len()
            || v1.index() >= surface.vertices.len()
            || v2.index() >= surface.vertices.len()
        {
            return Err(SurfaceTopologyError::InvalidSurfaceVertex(v0));
        }

        let p0 = surface.vertices[v0.index()].position;
        let p1 = surface.vertices[v1.index()].position;
        let p2 = surface.vertices[v2.index()].position;
        let area = triangle_signed_area(p0, p1, p2);
        if !area.is_finite() || area <= 1e-6 {
            return Err(SurfaceTopologyError::DegenerateTriangle {
                hex: face.owner_hex,
                sector: face.source.sector,
                triangle: face.source.triangle,
            });
        }

        if face.boundary.index() >= surface.half_edges.len() {
            return Err(SurfaceTopologyError::InvalidFaceBoundary {
                face: f_id,
                edge: face.boundary,
            });
        }

        let h0_id = face.boundary;
        let h0 = &surface.half_edges[h0_id.index()];
        let h1_id = h0.next;
        if h1_id.index() >= surface.half_edges.len() {
            return Err(SurfaceTopologyError::InvalidSurfaceHalfEdge(h1_id));
        }
        let h1 = &surface.half_edges[h1_id.index()];

        let h2_id = h1.next;
        if h2_id.index() >= surface.half_edges.len() {
            return Err(SurfaceTopologyError::InvalidSurfaceHalfEdge(h2_id));
        }
        let h2 = &surface.half_edges[h2_id.index()];

        if h2.next != h0_id {
            return Err(SurfaceTopologyError::InvalidHalfEdgeCycle { face: f_id });
        }

        if h0.incident_face != f_id || h1.incident_face != f_id || h2.incident_face != f_id {
            return Err(SurfaceTopologyError::HalfEdgeFaceMismatch {
                edge: h0_id,
                expected: f_id,
                actual: h0.incident_face,
            });
        }

        if h0.origin != v0 || h1.origin != v1 || h2.origin != v2 {
            return Err(SurfaceTopologyError::InvalidHalfEdgeCycle { face: f_id });
        }
    }

    // 3. Validate half-edges, reciprocity, and twins
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

        if next_he.prev != he_id || prev_he.next != he_id {
            return Err(SurfaceTopologyError::InvalidHalfEdgeCycle {
                face: he.incident_face,
            });
        }
        if next_he.origin != he.destination || prev_he.destination != he.origin {
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
            if twin_he.incident_face == he.incident_face {
                return Err(SurfaceTopologyError::HalfEdgeFaceMismatch {
                    edge: twin_id,
                    expected: SurfaceFaceId::new(usize::MAX),
                    actual: he.incident_face,
                });
            }
        }
    }

    // 4. Validate two-way hex_to_faces mapping completeness & strict uniqueness
    let mut mapped_faces = HashSet::with_capacity(surface.faces.len());
    for (&hex, faces) in &surface.hex_to_faces {
        for &f_id in faces {
            if f_id.index() >= surface.faces.len() {
                return Err(SurfaceTopologyError::InvalidSurfaceFace(f_id));
            }
            let face = &surface.faces[f_id.index()];
            if face.owner_hex != hex {
                return Err(SurfaceTopologyError::FaceHexMismatch {
                    expected: hex,
                    actual: face.owner_hex,
                });
            }
            if !mapped_faces.insert(f_id) {
                return Err(SurfaceTopologyError::InvalidSurfaceFace(f_id));
            }
        }
    }
    if mapped_faces.len() != surface.faces.len() {
        return Err(SurfaceTopologyError::InvalidSurfaceFace(
            SurfaceFaceId::new(mapped_faces.len()),
        ));
    }

    // 5. Validate stats consistency
    let paired_count = surface
        .half_edges
        .iter()
        .filter(|he| he.twin.is_some())
        .count();
    let boundary_count = surface
        .half_edges
        .iter()
        .filter(|he| he.twin.is_none())
        .count();

    if surface.stats.vertex_count != surface.vertices.len()
        || surface.stats.face_count != surface.faces.len()
        || surface.stats.half_edge_count != surface.half_edges.len()
        || surface.stats.paired_half_edge_count != paired_count
        || surface.stats.boundary_half_edge_count != boundary_count
        || paired_count + boundary_count != surface.half_edges.len()
    {
        return Err(SurfaceTopologyError::InvalidProvenance {
            vertex: SurfaceVertexId::new(0),
        });
    }

    Ok(())
}

/// Validates Fixed24 generator policy invariants (bounds & counts).
///
/// # Errors
/// Returns `SurfaceTopologyError` if sector/triangle bounds or tile counts violate Fixed24 policy.
pub fn validate_fixed24_surface_topology(
    surface: &SurfaceTopology,
    face_topology: &HexFaceTopology,
) -> Result<(), SurfaceTopologyError> {
    for face in &surface.faces {
        if face.source.sector >= 6 || face.source.triangle >= 4 {
            return Err(SurfaceTopologyError::DegenerateTriangle {
                hex: face.owner_hex,
                sector: face.source.sector,
                triangle: face.source.triangle,
            });
        }
    }

    for &hex in face_topology.hex_to_face.keys() {
        let faces = surface
            .hex_to_faces
            .get(&hex)
            .ok_or(SurfaceTopologyError::MissingFaceForTile(hex))?;
        if faces.len() != 24 {
            return Err(SurfaceTopologyError::DegenerateTriangle {
                hex,
                sector: 0,
                triangle: 0,
            });
        }
    }

    Ok(())
}
