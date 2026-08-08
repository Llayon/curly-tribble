// src/map/surface_topology/provenance_validation.rs
//! Pure validation logic for semantic coarse vertex ancestry against `HexFaceTopology`.

use crate::map::face_topology::HexFaceTopology;
use crate::map::surface_topology::types::{
    SurfaceTopology, SurfaceTopologyError, SurfaceVertexId, SurfaceVertexSource,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[allow(dead_code)]
pub struct SurfaceTopologyProvenanceValidationPlugin;

impl Plugin for SurfaceTopologyProvenanceValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Validates semantic coarse vertex ancestry and bit-exact arithmetic against `HexFaceTopology`.
///
/// # Errors
/// Returns `SurfaceTopologyError` if provenance ancestry, edge adjacency, or bit-exact position matches fail.
#[allow(clippy::too_many_lines)]
pub fn validate_surface_provenance(
    surface: &SurfaceTopology,
    face_topology: &HexFaceTopology,
) -> Result<(), SurfaceTopologyError> {
    let mut coarse_edges = HashSet::new();
    for face in &face_topology.faces {
        for i in 0..6 {
            let next = (i + 1) % 6;
            let va = face.vertices[i];
            let vb = face.vertices[next];
            let key = (va.min(vb), va.max(vb));
            coarse_edges.insert(key);
        }
    }

    let mut hex_centers = HashMap::with_capacity(face_topology.hex_to_face.len());
    for (&hex, &face_id) in &face_topology.hex_to_face {
        let face = face_topology
            .faces
            .get(face_id.index())
            .ok_or(SurfaceTopologyError::InvalidSourceFace { hex, face: face_id })?;
        let mut sum = Vec2::ZERO;
        for &v_id in &face.vertices {
            if v_id.index() >= face_topology.vertices.len() {
                return Err(SurfaceTopologyError::InvalidSourceVertex {
                    face: hex,
                    vertex: v_id,
                });
            }
            sum += face_topology.vertices[v_id.index()].position;
        }
        let center_pos = sum / 6.0;
        hex_centers.insert(hex, center_pos);
    }

    for (idx, vertex) in surface.vertices.iter().enumerate() {
        let v_id = SurfaceVertexId::new(idx);
        match &vertex.source {
            SurfaceVertexSource::HexCorner { source_vertex } => {
                if source_vertex.index() >= face_topology.vertices.len() {
                    return Err(SurfaceTopologyError::InvalidSourceVertex {
                        face: crate::map::HexCoord::new(0, 0),
                        vertex: *source_vertex,
                    });
                }
                let source_pos = face_topology.vertices[source_vertex.index()].position;
                if vertex.position.x.to_bits() != source_pos.x.to_bits()
                    || vertex.position.y.to_bits() != source_pos.y.to_bits()
                {
                    return Err(SurfaceTopologyError::InvalidProvenance { vertex: v_id });
                }
            }
            SurfaceVertexSource::HexEdgeMidpoint { source_a, source_b } => {
                if source_a.index() >= source_b.index() {
                    return Err(SurfaceTopologyError::InvalidProvenance { vertex: v_id });
                }
                if source_b.index() >= face_topology.vertices.len() {
                    return Err(SurfaceTopologyError::InvalidSourceVertex {
                        face: crate::map::HexCoord::new(0, 0),
                        vertex: *source_b,
                    });
                }
                let key = (*source_a, *source_b);
                if !coarse_edges.contains(&key) {
                    return Err(SurfaceTopologyError::InvalidProvenance { vertex: v_id });
                }
                let pos_a = face_topology.vertices[source_a.index()].position;
                let pos_b = face_topology.vertices[source_b.index()].position;
                let expected = 0.5 * (pos_a + pos_b);
                if vertex.position.x.to_bits() != expected.x.to_bits()
                    || vertex.position.y.to_bits() != expected.y.to_bits()
                {
                    return Err(SurfaceTopologyError::InvalidProvenance { vertex: v_id });
                }
            }
            SurfaceVertexSource::HexCenter { hex } => {
                let &center_pos = hex_centers
                    .get(hex)
                    .ok_or(SurfaceTopologyError::MissingFaceForTile(*hex))?;
                if vertex.position.x.to_bits() != center_pos.x.to_bits()
                    || vertex.position.y.to_bits() != center_pos.y.to_bits()
                {
                    return Err(SurfaceTopologyError::InvalidProvenance { vertex: v_id });
                }
            }
            SurfaceVertexSource::HexRadialMidpoint { hex, source_corner } => {
                let &face_id = face_topology
                    .hex_to_face
                    .get(hex)
                    .ok_or(SurfaceTopologyError::MissingFaceForTile(*hex))?;
                let face = &face_topology.faces[face_id.index()];
                if !face.vertices.contains(source_corner) {
                    return Err(SurfaceTopologyError::InvalidProvenance { vertex: v_id });
                }
                let &center_pos = hex_centers
                    .get(hex)
                    .ok_or(SurfaceTopologyError::MissingFaceForTile(*hex))?;
                let corner_pos = face_topology.vertices[source_corner.index()].position;
                let expected = 0.5 * (center_pos + corner_pos);
                if vertex.position.x.to_bits() != expected.x.to_bits()
                    || vertex.position.y.to_bits() != expected.y.to_bits()
                {
                    return Err(SurfaceTopologyError::InvalidProvenance { vertex: v_id });
                }
            }
        }
    }

    Ok(())
}
