//! Directed cliff edge binding between logical `EdgeCoord` and `HexFaceTopology` half-edges.

use crate::map::data::{CliffLowerSide, EdgeCoord, EdgeType, MapData};
use crate::map::face_topology::types::{FaceId, HalfEdgeId, HexFaceTopology, VertexId};
use bevy::prelude::*;

pub struct EdgeBindingPlugin;

impl Plugin for EdgeBindingPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundCliffEdge {
    pub logical_edge: EdgeCoord,
    pub face_a: FaceId,
    pub face_b: FaceId,
    pub half_edge_a: HalfEdgeId,
    pub half_edge_b: HalfEdgeId,
    pub lower_side: CliffLowerSide,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct BoundCliffEdges {
    pub edges: Vec<BoundCliffEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CliffBindingError {
    MissingTileA(EdgeCoord),
    MissingTileB(EdgeCoord),
    MissingFaceA(EdgeCoord),
    MissingFaceB(EdgeCoord),
    MissingAdjacency(EdgeCoord),
    MissingTwin(HalfEdgeId),
    NonReciprocalTwin {
        edge_a: HalfEdgeId,
        edge_b: HalfEdgeId,
    },
    IncidentFaceMismatch {
        edge: EdgeCoord,
        expected_face: FaceId,
        actual_face: FaceId,
    },
    ReversedEndpointMismatch {
        edge: EdgeCoord,
        origin_a: VertexId,
        dest_a: VertexId,
        origin_b: VertexId,
        dest_b: VertexId,
    },
}

/// Binds persistent logical cliff edges from `MapData` to reciprocal half-edges in `HexFaceTopology`.
///
/// # Errors
/// Returns `CliffBindingError` if any cliff edge cannot be uniquely resolved to an adjacent reciprocal twin pair in `HexFaceTopology`.
pub fn bind_cliff_edges(
    map_data: &MapData,
    topology: &HexFaceTopology,
) -> Result<BoundCliffEdges, CliffBindingError> {
    let mut sorted_cliff_entries: Vec<(EdgeCoord, CliffLowerSide)> = map_data
        .edges
        .iter()
        .filter(|(_, data)| data.edge_type == EdgeType::Cliff)
        .map(|(&edge, data)| (edge, data.cliff_lower_side))
        .collect();

    sorted_cliff_entries.sort_by_key(|(edge, _)| (edge.a, edge.b));

    let mut bound_edges = Vec::with_capacity(sorted_cliff_entries.len());

    for (logical_edge, lower_side) in sorted_cliff_entries {
        if !map_data.tiles.contains_key(&logical_edge.a) {
            return Err(CliffBindingError::MissingTileA(logical_edge));
        }
        if !map_data.tiles.contains_key(&logical_edge.b) {
            return Err(CliffBindingError::MissingTileB(logical_edge));
        }

        let &face_a = topology
            .hex_to_face
            .get(&logical_edge.a)
            .ok_or(CliffBindingError::MissingFaceA(logical_edge))?;
        let &face_b = topology
            .hex_to_face
            .get(&logical_edge.b)
            .ok_or(CliffBindingError::MissingFaceB(logical_edge))?;

        let face_a_obj = topology
            .faces
            .get(face_a.index())
            .ok_or(CliffBindingError::MissingFaceA(logical_edge))?;
        let _face_b_obj = topology
            .faces
            .get(face_b.index())
            .ok_or(CliffBindingError::MissingFaceB(logical_edge))?;

        let mut matched_pair = None;
        let mut curr_he_id = face_a_obj.boundary;
        for _ in 0..6 {
            let Some(he_a) = topology.half_edges.get(curr_he_id.index()) else {
                break;
            };
            if let Some(twin_id) = he_a.twin {
                if let Some(twin) = topology.half_edges.get(twin_id.index()) {
                    if twin.incident_face == face_b {
                        matched_pair = Some((curr_he_id, twin_id));
                        break;
                    }
                }
            }
            curr_he_id = he_a.next;
        }

        let (bound_he_a, bound_he_b) =
            matched_pair.ok_or(CliffBindingError::MissingAdjacency(logical_edge))?;

        let he_a = topology
            .half_edges
            .get(bound_he_a.index())
            .ok_or(CliffBindingError::MissingAdjacency(logical_edge))?;
        let he_b = topology
            .half_edges
            .get(bound_he_b.index())
            .ok_or(CliffBindingError::MissingAdjacency(logical_edge))?;

        if he_b.twin != Some(bound_he_a) {
            return Err(CliffBindingError::NonReciprocalTwin {
                edge_a: bound_he_a,
                edge_b: bound_he_b,
            });
        }

        if he_a.incident_face != face_a {
            return Err(CliffBindingError::IncidentFaceMismatch {
                edge: logical_edge,
                expected_face: face_a,
                actual_face: he_a.incident_face,
            });
        }
        if he_b.incident_face != face_b {
            return Err(CliffBindingError::IncidentFaceMismatch {
                edge: logical_edge,
                expected_face: face_b,
                actual_face: he_b.incident_face,
            });
        }

        if he_a.origin != he_b.destination || he_a.destination != he_b.origin {
            return Err(CliffBindingError::ReversedEndpointMismatch {
                edge: logical_edge,
                origin_a: he_a.origin,
                dest_a: he_a.destination,
                origin_b: he_b.origin,
                dest_b: he_b.destination,
            });
        }

        bound_edges.push(BoundCliffEdge {
            logical_edge,
            face_a,
            face_b,
            half_edge_a: bound_he_a,
            half_edge_b: bound_he_b,
            lower_side,
        });
    }

    Ok(BoundCliffEdges { edges: bound_edges })
}
