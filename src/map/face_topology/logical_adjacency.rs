//! Exact logical-neighbor and non-neighbor edge invariants.
use crate::map::data::MapData;
use crate::map::face_topology::types::{
    FaceId, HalfEdge, HalfEdgeId, HexFace, HexFaceTopology, HexFaceTopologyError, VertexId,
};
use std::collections::{HashMap, HashSet};

type EdgeKey = (usize, usize);

fn invalid(message: impl Into<String>) -> HexFaceTopologyError {
    HexFaceTopologyError::ValidationFailed(message.into())
}

fn edge_key(origin: VertexId, destination: VertexId) -> EdgeKey {
    if origin.index() < destination.index() {
        (origin.index(), destination.index())
    } else {
        (destination.index(), origin.index())
    }
}

fn get_face(topology: &HexFaceTopology, face_id: FaceId) -> Result<&HexFace, HexFaceTopologyError> {
    topology
        .faces
        .get(face_id.index())
        .ok_or_else(|| invalid(format!("Invalid face id {face_id:?}")))
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

fn face_edge_keys(face: &HexFace) -> Vec<EdgeKey> {
    (0..6)
        .map(|index| edge_key(face.vertices[index], face.vertices[(index + 1) % 6]))
        .collect()
}

fn shared_face_edge_keys(face: &HexFace, shared: &HashSet<VertexId>) -> Vec<EdgeKey> {
    face_edge_keys(face)
        .into_iter()
        .filter(|(origin, destination)| {
            shared.contains(&VertexId::new(*origin))
                && shared.contains(&VertexId::new(*destination))
        })
        .collect()
}

fn shared_half_edges(
    topology: &HexFaceTopology,
    face_id: FaceId,
    shared: &HashSet<VertexId>,
) -> Vec<HalfEdgeId> {
    topology
        .half_edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| {
            (edge.incident_face == face_id
                && shared.contains(&edge.origin)
                && shared.contains(&edge.destination))
            .then_some(HalfEdgeId::new(index))
        })
        .collect()
}

fn mapped_face(
    topology: &HexFaceTopology,
    coord: crate::map::HexCoord,
) -> Result<FaceId, HexFaceTopologyError> {
    topology
        .hex_to_face
        .get(&coord)
        .copied()
        .ok_or_else(|| invalid(format!("Missing face mapping for {coord:?}")))
}

#[allow(clippy::similar_names)]
fn validate_neighbor_pair(
    topology: &HexFaceTopology,
    coord_a: crate::map::HexCoord,
    coord_b: crate::map::HexCoord,
) -> Result<(), HexFaceTopologyError> {
    let face_a_id = mapped_face(topology, coord_a)?;
    let face_b_id = mapped_face(topology, coord_b)?;
    let face_a = get_face(topology, face_a_id)?;
    let face_b = get_face(topology, face_b_id)?;
    let vertices_a: HashSet<_> = face_a.vertices.iter().copied().collect();
    let vertices_b: HashSet<_> = face_b.vertices.iter().copied().collect();
    let shared: HashSet<_> = vertices_a.intersection(&vertices_b).copied().collect();

    if shared.len() != 2 {
        return Err(invalid(format!(
            "Neighbor faces {face_a_id:?}/{face_b_id:?} ({coord_a:?}/{coord_b:?}) share {} vertices, expected 2",
            shared.len()
        )));
    }

    let face_edges_a = shared_face_edge_keys(face_a, &shared);
    let face_edges_b = shared_face_edge_keys(face_b, &shared);
    if face_edges_a.len() != 1 || face_edges_b.len() != 1 || face_edges_a[0] != face_edges_b[0] {
        return Err(invalid(format!(
            "Neighbor faces {face_a_id:?}/{face_b_id:?} share {} and {} logical edges, expected one identical edge",
            face_edges_a.len(),
            face_edges_b.len()
        )));
    }

    let half_edges_a = shared_half_edges(topology, face_a_id, &shared);
    let half_edges_b = shared_half_edges(topology, face_b_id, &shared);
    if half_edges_a.len() != 1 || half_edges_b.len() != 1 {
        return Err(invalid(format!(
            "Neighbor faces {face_a_id:?}/{face_b_id:?} have {} and {} shared half-edges, expected one each",
            half_edges_a.len(),
            half_edges_b.len()
        )));
    }
    let edge_a_id = half_edges_a[0];
    let edge_b_id = half_edges_b[0];
    let edge_a = get_edge(topology, edge_a_id)?;
    let edge_b = get_edge(topology, edge_b_id)?;
    if edge_a.origin != edge_b.destination || edge_a.destination != edge_b.origin {
        return Err(invalid(format!(
            "Neighbor edges {edge_a_id:?}/{edge_b_id:?} for {coord_a:?}/{coord_b:?} are not reversed"
        )));
    }

    match (edge_a.twin, edge_b.twin) {
        (None, None) => Err(invalid(format!(
            "Internal edge pair {edge_a_id:?}/{edge_b_id:?} for {coord_a:?}/{coord_b:?} is incorrectly boundary"
        ))),
        (None, Some(_)) | (Some(_), None) => Err(invalid(format!(
            "Neighbor edges {edge_a_id:?}/{edge_b_id:?} for {coord_a:?}/{coord_b:?} have one missing twin"
        ))),
        (Some(twin_a), Some(twin_b)) if twin_a == edge_b_id && twin_b == edge_a_id => Ok(()),
        (Some(twin_a), Some(twin_b)) => Err(invalid(format!(
            "Neighbor edges {edge_a_id:?}/{edge_b_id:?} for {coord_a:?}/{coord_b:?} have asymmetric twins {twin_a:?}/{twin_b:?}"
        ))),
    }
}

fn validate_face_edge_owners(topology: &HexFaceTopology) -> Result<(), HexFaceTopologyError> {
    let mut face_owners: HashMap<EdgeKey, Vec<FaceId>> = HashMap::new();
    for (index, face) in topology.faces.iter().enumerate() {
        let face_id = FaceId::new(index);
        for key in face_edge_keys(face) {
            face_owners.entry(key).or_default().push(face_id);
        }
    }
    for (key, owners) in face_owners {
        let unique_owners: HashSet<_> = owners.iter().copied().collect();
        let owner_ids: Vec<_> = unique_owners.into_iter().collect();
        for (index, &face_a_id) in owner_ids.iter().enumerate() {
            for &face_b_id in owner_ids.iter().skip(index + 1) {
                let face_a = get_face(topology, face_a_id)?;
                let face_b = get_face(topology, face_b_id)?;
                if !face_a.hex.neighbors().contains(&face_b.hex) {
                    return Err(invalid(format!(
                        "Non-neighbor faces {face_a_id:?}/{face_b_id:?} ({:?}/{:?}) share edge {key:?}",
                        face_a.hex, face_b.hex
                    )));
                }
            }
        }
    }

    let mut half_edge_owners: HashMap<EdgeKey, Vec<(FaceId, HalfEdgeId)>> = HashMap::new();
    for (index, edge) in topology.half_edges.iter().enumerate() {
        half_edge_owners
            .entry(edge_key(edge.origin, edge.destination))
            .or_default()
            .push((edge.incident_face, HalfEdgeId::new(index)));
    }
    for (key, owners) in half_edge_owners {
        for (index, &(face_a_id, edge_a_id)) in owners.iter().enumerate() {
            for &(face_b_id, edge_b_id) in owners.iter().skip(index + 1) {
                if face_a_id == face_b_id {
                    continue;
                }
                let face_a = get_face(topology, face_a_id)?;
                let face_b = get_face(topology, face_b_id)?;
                if !face_a.hex.neighbors().contains(&face_b.hex) {
                    return Err(invalid(format!(
                        "Non-neighbor half-edges {edge_a_id:?}/{edge_b_id:?} on faces {face_a_id:?}/{face_b_id:?} share edge {key:?}"
                    )));
                }
                let edge_a = get_edge(topology, edge_a_id)?;
                let edge_b = get_edge(topology, edge_b_id)?;
                if edge_a.twin.is_none() || edge_b.twin.is_none() {
                    return Err(invalid(format!(
                        "Boundary edge {key:?} on faces {face_a_id:?}/{face_b_id:?} is a missing internal twin"
                    )));
                }
            }
        }
    }

    Ok(())
}

fn validate_all_twin_links(topology: &HexFaceTopology) -> Result<(), HexFaceTopologyError> {
    for (index, edge) in topology.half_edges.iter().enumerate() {
        let Some(twin_id) = edge.twin else {
            continue;
        };
        let edge_id = HalfEdgeId::new(index);
        let twin = get_edge(topology, twin_id)?;
        let face_a = get_face(topology, edge.incident_face)?;
        let face_b = get_face(topology, twin.incident_face)?;
        if !face_a.hex.neighbors().contains(&face_b.hex) {
            return Err(invalid(format!(
                "Twin edge {edge_id:?}/{twin_id:?} connects non-neighbor faces {:?}/{:?}",
                face_a.hex, face_b.hex
            )));
        }
        if twin.origin != edge.destination || twin.destination != edge.origin {
            return Err(invalid(format!(
                "Twin edge {edge_id:?}/{twin_id:?} has same-direction endpoints"
            )));
        }
        if twin.twin != Some(edge_id) {
            return Err(HexFaceTopologyError::InconsistentTwin {
                edge: edge_id,
                twin: twin_id,
            });
        }
    }
    Ok(())
}

/// Validates exact shared-edge relationships for all logical neighbors.
///
/// # Errors
/// Returns an error for missing, duplicated, non-reversed, asymmetric, or
/// non-logical shared edges and twins.
pub fn validate_logical_adjacency(
    topology: &HexFaceTopology,
    map_data: &MapData,
) -> Result<(), HexFaceTopologyError> {
    for &coord_a in map_data.tiles.keys() {
        for coord_b in coord_a.neighbors() {
            if coord_b <= coord_a || !map_data.tiles.contains_key(&coord_b) {
                continue;
            }
            validate_neighbor_pair(topology, coord_a, coord_b)?;
        }
    }
    validate_all_twin_links(topology)?;
    validate_face_edge_owners(topology)
}
