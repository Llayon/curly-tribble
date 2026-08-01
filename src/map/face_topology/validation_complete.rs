/// Complete final validation for generated hex face topology.
use crate::map::data::MapData;
use crate::map::face_topology::types::{
    FaceId, HalfEdge, HalfEdgeId, HexFace, HexFaceTopology, HexFaceTopologyError, VertexId,
};
use crate::map::face_topology::validation::validate_face_geometry;
use bevy::prelude::Vec2;
use std::collections::HashSet;

fn invalid(message: impl Into<String>) -> HexFaceTopologyError {
    HexFaceTopologyError::ValidationFailed(message.into())
}

fn get_edge(
    topology: &HexFaceTopology,
    edge_id: HalfEdgeId,
) -> Result<&HalfEdge, HexFaceTopologyError> {
    topology
        .half_edges
        .get(edge_id.index())
        .ok_or_else(|| invalid(format!("Invalid half-edge id {edge_id:?}")))
}

fn get_face(topology: &HexFaceTopology, face_id: FaceId) -> Result<&HexFace, HexFaceTopologyError> {
    topology
        .faces
        .get(face_id.index())
        .ok_or_else(|| invalid(format!("Invalid face id {face_id:?}")))
}

fn validate_boundary_cycles(
    topology: &HexFaceTopology,
    face_id: FaceId,
    face_vertices: &[VertexId; 6],
    edge_seen: &mut [bool],
    directed_edges: &mut HashSet<(usize, usize)>,
) -> Result<(), HexFaceTopologyError> {
    let face = get_face(topology, face_id)?;
    let mut next_seen = HashSet::new();
    let mut current = face.boundary;
    for step in 0..6 {
        if !next_seen.insert(current) {
            return Err(invalid(format!(
                "Face {face_id:?} Next cycle repeats early"
            )));
        }
        let edge = get_edge(topology, current)?;
        if edge.incident_face != face_id {
            return Err(invalid(format!("Edge {current:?} has wrong incident face")));
        }
        if edge.origin != face_vertices[step] || edge.destination != face_vertices[(step + 1) % 6] {
            return Err(invalid(format!(
                "Face {face_id:?} edge order is inconsistent"
            )));
        }
        if edge_seen[current.index()] {
            return Err(invalid(format!(
                "Half-edge {current:?} belongs to multiple faces"
            )));
        }
        edge_seen[current.index()] = true;
        if !directed_edges.insert((edge.origin.index(), edge.destination.index())) {
            return Err(invalid(format!(
                "Duplicate directed edge ({}, {})",
                edge.origin.index(),
                edge.destination.index()
            )));
        }
        if get_edge(topology, edge.next)?.prev != current {
            return Err(invalid(format!(
                "Edge {current:?} Next/Prev is inconsistent"
            )));
        }
        current = edge.next;
    }
    if current != face.boundary || next_seen.len() != 6 {
        return Err(invalid(format!(
            "Face {face_id:?} Next cycle is not six edges"
        )));
    }

    let mut prev_seen = HashSet::new();
    current = face.boundary;
    for _ in 0..6 {
        if !prev_seen.insert(current) {
            return Err(invalid(format!(
                "Face {face_id:?} Prev cycle repeats early"
            )));
        }
        let edge = get_edge(topology, current)?;
        if edge.incident_face != face_id {
            return Err(invalid(format!("Edge {current:?} has wrong incident face")));
        }
        if get_edge(topology, edge.prev)?.next != current {
            return Err(invalid(format!(
                "Edge {current:?} Prev/Next is inconsistent"
            )));
        }
        current = edge.prev;
    }
    if current != face.boundary || prev_seen.len() != 6 {
        return Err(invalid(format!(
            "Face {face_id:?} Prev cycle is not six edges"
        )));
    }
    Ok(())
}

/// Performs final validation of every generated topology invariant.
///
/// # Errors
/// Returns a meaningful validation error instead of indexing malformed data.
#[allow(clippy::too_many_lines)]
pub fn validate_complete_topology(
    topology: &HexFaceTopology,
    map_data: &MapData,
) -> Result<(), HexFaceTopologyError> {
    if topology.faces.len() != map_data.tiles.len() {
        return Err(invalid(format!(
            "Expected one face per tile: {} faces for {} tiles",
            topology.faces.len(),
            map_data.tiles.len()
        )));
    }
    let expected_edges = topology
        .faces
        .len()
        .checked_mul(6)
        .ok_or_else(|| invalid("Face count overflow while calculating edge count"))?;
    if topology.half_edges.len() != expected_edges {
        return Err(invalid(format!(
            "Expected six half-edges per face: {} edges for {} faces",
            topology.half_edges.len(),
            topology.faces.len()
        )));
    }
    if topology.hex_to_face.len() != map_data.tiles.len() {
        return Err(invalid(format!(
            "hex_to_face is not complete: {} mappings for {} tiles",
            topology.hex_to_face.len(),
            map_data.tiles.len()
        )));
    }

    let mut mapped_faces = vec![false; topology.faces.len()];
    for (&coord, &face_id) in &topology.hex_to_face {
        let Some(face) = topology.faces.get(face_id.index()) else {
            return Err(invalid(format!(
                "Mapping for {coord:?} has invalid face id"
            )));
        };
        if mapped_faces[face_id.index()] {
            return Err(invalid(format!(
                "Face {face_id:?} is mapped more than once"
            )));
        }
        if face.hex != coord || !map_data.tiles.contains_key(&coord) {
            return Err(invalid(format!(
                "Invalid hex_to_face mapping for {coord:?}"
            )));
        }
        mapped_faces[face_id.index()] = true;
    }
    for (index, (&mapped, face)) in mapped_faces.iter().zip(&topology.faces).enumerate() {
        if !mapped || !map_data.tiles.contains_key(&face.hex) {
            return Err(invalid(format!(
                "Face {index} is outside the map bijection"
            )));
        }
    }

    for (index, edge) in topology.half_edges.iter().enumerate() {
        if edge.origin.index() >= topology.vertices.len()
            || edge.destination.index() >= topology.vertices.len()
        {
            return Err(invalid(format!(
                "Half-edge {index} references invalid vertices"
            )));
        }
        if edge.next.index() >= topology.half_edges.len()
            || edge.prev.index() >= topology.half_edges.len()
        {
            return Err(invalid(format!(
                "Half-edge {index} references invalid cycle edges"
            )));
        }
        if edge.incident_face.index() >= topology.faces.len() {
            return Err(invalid(format!(
                "Half-edge {index} references invalid face"
            )));
        }
        if edge
            .twin
            .is_some_and(|twin| twin.index() >= topology.half_edges.len())
        {
            return Err(invalid(format!(
                "Half-edge {index} references invalid twin"
            )));
        }
    }

    let mut edge_seen = vec![false; topology.half_edges.len()];
    let mut directed_edges = HashSet::new();
    for (index, face) in topology.faces.iter().enumerate() {
        let face_id = FaceId::new(index);
        let unique_vertices: HashSet<_> = face.vertices.iter().copied().collect();
        if unique_vertices.len() != 6 {
            return Err(invalid(format!(
                "Face {index} does not have six unique vertices"
            )));
        }
        let mut points = [Vec2::ZERO; 6];
        for (point_index, &vertex_id) in face.vertices.iter().enumerate() {
            let Some(vertex) = topology.vertices.get(vertex_id.index()) else {
                return Err(invalid(format!("Face {index} references invalid vertex")));
            };
            points[point_index] = vertex.position;
        }
        validate_face_geometry(&points, face_id)?;
        validate_boundary_cycles(
            topology,
            face_id,
            &face.vertices,
            &mut edge_seen,
            &mut directed_edges,
        )?;
    }
    if edge_seen.iter().any(|seen| !seen) {
        return Err(invalid("Topology contains an unreferenced half-edge"));
    }

    let mut paired_edges = 0;
    let mut border_edges = 0;
    for (index, edge) in topology.half_edges.iter().enumerate() {
        let edge_id = HalfEdgeId::new(index);
        let face_a = get_face(topology, edge.incident_face)?;
        if let Some(twin_id) = edge.twin {
            paired_edges += 1;
            let twin = get_edge(topology, twin_id)?;
            if twin.twin != Some(edge_id) {
                return Err(HexFaceTopologyError::InconsistentTwin {
                    edge: edge_id,
                    twin: twin_id,
                });
            }
            if twin.origin != edge.destination || twin.destination != edge.origin {
                return Err(invalid(format!(
                    "Twin {twin_id:?} does not reverse {edge_id:?}"
                )));
            }
            let face_b = get_face(topology, twin.incident_face)?;
            if !face_a.hex.neighbors().contains(&face_b.hex) {
                return Err(invalid(format!(
                    "Twin faces {:?} and {:?} are not logical neighbors",
                    face_a.hex, face_b.hex
                )));
            }
        } else {
            border_edges += 1;
        }
    }
    let stats_edge_total = topology
        .stats
        .paired_edge_count
        .checked_mul(2)
        .and_then(|paired| paired.checked_add(topology.stats.border_edge_count));
    if paired_edges % 2 != 0
        || paired_edges / 2 != topology.stats.paired_edge_count
        || border_edges != topology.stats.border_edge_count
        || topology.stats.half_edge_count != topology.half_edges.len()
        || stats_edge_total != Some(topology.half_edges.len())
    {
        return Err(invalid(format!(
            "Edge statistics are inconsistent: paired={}, border={}, half_edges={}",
            topology.stats.paired_edge_count,
            topology.stats.border_edge_count,
            topology.half_edges.len()
        )));
    }

    crate::map::face_topology::validation_twins::validate_neighbor_twins(topology, map_data)?;
    crate::map::face_topology::validation_twins::validate_border_edges(topology, map_data)?;
    Ok(())
}
