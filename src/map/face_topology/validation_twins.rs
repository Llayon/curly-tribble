/// Logical-neighbor and border-edge validation for hex face topology.
use crate::map::data::MapData;
use crate::map::face_topology::types::{
    FaceId, HalfEdge, HalfEdgeId, HexFaceTopology, HexFaceTopologyError,
};
use std::collections::{HashMap, HashSet};

fn invalid(message: impl Into<String>) -> HexFaceTopologyError {
    HexFaceTopologyError::ValidationFailed(message.into())
}

fn undirected_edge_key(a: usize, b: usize) -> (usize, usize) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

fn edges_for_face(
    topology: &HexFaceTopology,
    face_id: FaceId,
) -> Result<Vec<HalfEdgeId>, HexFaceTopologyError> {
    let Some(face) = topology.faces.get(face_id.index()) else {
        return Err(invalid(format!("Invalid face id {face_id:?}")));
    };
    let mut result = Vec::with_capacity(6);
    let mut seen = HashSet::new();
    let mut current = face.boundary;
    for _ in 0..6 {
        if !seen.insert(current) {
            return Err(invalid(format!("Face {face_id:?} cycle repeats early")));
        }
        let Some(edge) = topology.half_edges.get(current.index()) else {
            return Err(invalid(format!("Invalid edge id {current:?}")));
        };
        if edge.incident_face != face_id {
            return Err(invalid(format!(
                "Edge {current:?} has the wrong incident face"
            )));
        }
        result.push(current);
        current = edge.next;
    }
    if current != face.boundary || result.len() != 6 {
        return Err(invalid(format!(
            "Face {face_id:?} does not have a six-edge cycle"
        )));
    }
    Ok(result)
}

fn edge_ref(
    topology: &HexFaceTopology,
    edge_id: HalfEdgeId,
) -> Result<&HalfEdge, HexFaceTopologyError> {
    topology
        .half_edges
        .get(edge_id.index())
        .ok_or_else(|| invalid(format!("Invalid edge id {edge_id:?}")))
}

/// For every logical neighbor pair in `MapData`, confirm a symmetric reversed twin.
///
/// # Errors
/// Returns an error if a map neighbor is unmapped or lacks a symmetric twin.
pub fn validate_neighbor_twins(
    topology: &HexFaceTopology,
    map_data: &MapData,
) -> Result<(), HexFaceTopologyError> {
    for &coord in map_data.tiles.keys() {
        let Some(&face_id) = topology.hex_to_face.get(&coord) else {
            return Err(invalid(format!("Missing face mapping for {coord:?}")));
        };
        let face_edges = edges_for_face(topology, face_id)?;
        for neighbor in coord.neighbors() {
            if neighbor <= coord || !map_data.tiles.contains_key(&neighbor) {
                continue;
            }
            let Some(&neighbor_face_id) = topology.hex_to_face.get(&neighbor) else {
                return Err(invalid(format!("Missing face mapping for {neighbor:?}")));
            };
            let mut found = false;
            for edge_id in &face_edges {
                let edge = edge_ref(topology, *edge_id)?;
                let Some(twin_id) = edge.twin else {
                    continue;
                };
                let twin = edge_ref(topology, twin_id)?;
                if twin.incident_face == neighbor_face_id {
                    if twin.origin != edge.destination || twin.destination != edge.origin {
                        return Err(invalid(format!(
                            "Twin {twin_id:?} does not reverse edge {edge_id:?}"
                        )));
                    }
                    if twin.twin != Some(*edge_id) {
                        return Err(HexFaceTopologyError::InconsistentTwin {
                            edge: *edge_id,
                            twin: twin_id,
                        });
                    }
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(invalid(format!(
                    "Neighbor pair {coord:?}/{neighbor:?} has no symmetric twin"
                )));
            }
        }
    }
    Ok(())
}

fn logical_neighbor_for_edge(
    topology: &HexFaceTopology,
    map_data: &MapData,
    face_id: FaceId,
    edge: &HalfEdge,
) -> Result<Option<FaceId>, HexFaceTopologyError> {
    let Some(face) = topology.faces.get(face_id.index()) else {
        return Err(invalid(format!("Invalid face id {face_id:?}")));
    };
    for neighbor in face.hex.neighbors() {
        if !map_data.tiles.contains_key(&neighbor) {
            continue;
        }
        let Some(&neighbor_face_id) = topology.hex_to_face.get(&neighbor) else {
            return Err(invalid(format!("Missing face mapping for {neighbor:?}")));
        };
        for neighbor_edge_id in edges_for_face(topology, neighbor_face_id)? {
            let neighbor_edge = edge_ref(topology, neighbor_edge_id)?;
            if neighbor_edge.origin == edge.destination && neighbor_edge.destination == edge.origin
            {
                return Ok(Some(neighbor_face_id));
            }
        }
    }
    Ok(None)
}

/// Confirm every border edge has no twin and every logical internal edge has one.
///
/// # Errors
/// Returns an error if an internal edge lacks a twin or a border edge has one.
pub fn validate_border_edges(
    topology: &HexFaceTopology,
    map_data: &MapData,
) -> Result<(), HexFaceTopologyError> {
    let mut undirected_counts = HashMap::new();
    for edge in &topology.half_edges {
        *undirected_counts
            .entry(undirected_edge_key(
                edge.origin.index(),
                edge.destination.index(),
            ))
            .or_insert(0usize) += 1;
    }

    for (edge_index, edge) in topology.half_edges.iter().enumerate() {
        let face_id = edge.incident_face;
        let expected_neighbor = logical_neighbor_for_edge(topology, map_data, face_id, edge)?;
        let actual_twin_face = edge.twin.map(|twin_id| {
            topology
                .half_edges
                .get(twin_id.index())
                .map(|twin| twin.incident_face)
        });
        match (expected_neighbor, actual_twin_face.flatten()) {
            (Some(expected), Some(actual)) if expected == actual => {}
            (Some(_), None) => {
                return Err(invalid(format!(
                    "Internal edge {edge_index} is missing its twin"
                )));
            }
            (Some(expected), Some(actual)) => {
                return Err(invalid(format!(
                    "Edge {edge_index} points to face {actual:?}, expected {expected:?}"
                )));
            }
            (None, None) => {
                let key = undirected_edge_key(edge.origin.index(), edge.destination.index());
                if undirected_counts.get(&key).copied().unwrap_or(0) != 1 {
                    return Err(invalid(format!("Border edge {edge_index} is duplicated")));
                }
            }
            (None, Some(actual)) => {
                return Err(invalid(format!(
                    "Border edge {edge_index} unexpectedly has twin face {actual:?}"
                )));
            }
        }
    }
    Ok(())
}
