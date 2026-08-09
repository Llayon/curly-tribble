// src/map/height_graph/validation_completeness.rs
//! Completeness proofs for non-cliff continuity and cliff relations in `HeightConstraintGraph`.

use crate::map::height_constraints::types::HeightConstraintSet;
use crate::map::height_graph::types::{HeightConstraintGraph, HeightGraphBuildError};
use crate::map::surface_topology::types::{SurfaceHalfEdgeId, SurfaceTopology};
use bevy::prelude::*;
use std::collections::{BTreeMap, HashSet};

#[allow(dead_code)]
pub struct HeightGraphValidationCompletenessPlugin;

impl Plugin for HeightGraphValidationCompletenessPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Validates non-cliff half-edge continuity cuts.
///
/// # Errors
/// Returns `HeightGraphBuildError` if twin height nodes mismatch across non-cliff half-edges.
#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation
)]
pub fn validate_non_cliff_continuity(
    graph: &HeightConstraintGraph,
    surface: &SurfaceTopology,
    constraints: &HeightConstraintSet,
) -> Result<(), HeightGraphBuildError> {
    let barrier_half_edges: HashSet<SurfaceHalfEdgeId> = constraints
        .cliffs
        .iter()
        .flat_map(|c| {
            c.segments
                .iter()
                .flat_map(|s| [s.half_edge_a, s.half_edge_b])
        })
        .collect();

    for (he_idx, half_edge) in surface.half_edges.iter().enumerate() {
        let he_id = SurfaceHalfEdgeId::new(he_idx);
        let Some(twin_id) = half_edge.twin else {
            continue;
        };

        if he_id.index() >= twin_id.index() {
            continue;
        }

        if barrier_half_edges.contains(&he_id) || barrier_half_edges.contains(&twin_id) {
            continue;
        }

        let twin = surface
            .half_edges
            .get(twin_id.index())
            .ok_or(HeightGraphBuildError::MissingTwin(twin_id))?;

        let face_a_id = half_edge.incident_face;
        let face_b_id = twin.incident_face;

        let face_a_ref = surface
            .faces
            .get(face_a_id.index())
            .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_a_id))?;
        let face_b_ref = surface
            .faces
            .get(face_b_id.index())
            .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_b_id))?;

        let u_vert = half_edge.origin;
        let v_vert = twin.origin;

        let corner_a_u = face_a_ref
            .vertices
            .iter()
            .position(|&v| v == u_vert)
            .ok_or(HeightGraphBuildError::FaceMissingVertex {
                face: face_a_id,
                vertex: u_vert,
            })?;
        let corner_b_u = face_b_ref
            .vertices
            .iter()
            .position(|&v| v == u_vert)
            .ok_or(HeightGraphBuildError::FaceMissingVertex {
                face: face_b_id,
                vertex: u_vert,
            })?;

        let node_a_u = graph.face_nodes[face_a_id.index()][corner_a_u];
        let node_b_u = graph.face_nodes[face_b_id.index()][corner_b_u];

        if node_a_u != node_b_u {
            return Err(HeightGraphBuildError::NonReciprocalTwin {
                a: he_id,
                b: twin_id,
            });
        }

        let corner_a_v = face_a_ref
            .vertices
            .iter()
            .position(|&v| v == v_vert)
            .ok_or(HeightGraphBuildError::FaceMissingVertex {
                face: face_a_id,
                vertex: v_vert,
            })?;
        let corner_b_v = face_b_ref
            .vertices
            .iter()
            .position(|&v| v == v_vert)
            .ok_or(HeightGraphBuildError::FaceMissingVertex {
                face: face_b_id,
                vertex: v_vert,
            })?;

        let node_a_v = graph.face_nodes[face_a_id.index()][corner_a_v];
        let node_b_v = graph.face_nodes[face_b_id.index()][corner_b_v];

        if node_a_v != node_b_v {
            return Err(HeightGraphBuildError::NonReciprocalTwin {
                a: he_id,
                b: twin_id,
            });
        }
    }

    Ok(())
}

/// Validates exact 1:1 cliff relations completeness and node mapping.
///
/// # Errors
/// Returns `HeightGraphBuildError` if reconstructed cliff relations mismatch graph cliff relations.
#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation
)]
pub fn validate_cliff_relations_completeness(
    graph: &HeightConstraintGraph,
    surface: &SurfaceTopology,
    constraints: &HeightConstraintSet,
) -> Result<(), HeightGraphBuildError> {
    let mut expected_map: BTreeMap<
        (
            crate::map::data::EdgeCoord,
            crate::map::surface_topology::types::SurfaceVertexId,
        ),
        crate::map::height_graph::types::CliffNodeRelation,
    > = BTreeMap::new();

    for cliff in &constraints.cliffs {
        for seg in &cliff.segments {
            let he_a = surface.half_edges.get(seg.half_edge_a.index()).ok_or(
                HeightGraphBuildError::InvalidSurfaceHalfEdge(seg.half_edge_a),
            )?;
            let he_b = surface.half_edges.get(seg.half_edge_b.index()).ok_or(
                HeightGraphBuildError::InvalidSurfaceHalfEdge(seg.half_edge_b),
            )?;

            let face_a_id = he_a.incident_face;
            let face_b_id = he_b.incident_face;

            let face_a_ref = surface
                .faces
                .get(face_a_id.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_a_id))?;
            let face_b_ref = surface
                .faces
                .get(face_b_id.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_b_id))?;

            for &vert_id in &[he_a.origin, he_a.destination] {
                let corner_a_v = face_a_ref
                    .vertices
                    .iter()
                    .position(|&v| v == vert_id)
                    .ok_or(HeightGraphBuildError::FaceMissingVertex {
                        face: face_a_id,
                        vertex: vert_id,
                    })?;
                let corner_b_v = face_b_ref
                    .vertices
                    .iter()
                    .position(|&v| v == vert_id)
                    .ok_or(HeightGraphBuildError::FaceMissingVertex {
                        face: face_b_id,
                        vertex: vert_id,
                    })?;

                let node_a_v = graph.face_nodes[face_a_id.index()][corner_a_v];
                let node_b_v = graph.face_nodes[face_b_id.index()][corner_b_v];

                let expected_rel = crate::map::height_graph::types::CliffNodeRelation {
                    logical_edge: cliff.logical_edge,
                    surface_vertex: vert_id,
                    node_a: node_a_v,
                    node_b: node_b_v,
                    lower_side: cliff.lower_side,
                };

                let key = (cliff.logical_edge, vert_id);
                if let Some(existing) = expected_map.get(&key) {
                    if existing != &expected_rel {
                        return Err(HeightGraphBuildError::InconsistentCliffVertexRelation {
                            edge: cliff.logical_edge,
                            vertex: vert_id,
                        });
                    }
                } else {
                    expected_map.insert(key, expected_rel);
                }
            }
        }
    }

    let mut expected_relations: Vec<_> = expected_map.into_values().collect();
    expected_relations.sort_by(|a, b| {
        a.logical_edge
            .cmp(&b.logical_edge)
            .then_with(|| a.surface_vertex.cmp(&b.surface_vertex))
            .then_with(|| a.node_a.cmp(&b.node_a))
            .then_with(|| a.node_b.cmp(&b.node_b))
    });

    if graph.cliff_relations != expected_relations {
        return Err(HeightGraphBuildError::CliffRelationMismatch {
            edge: crate::map::data::EdgeCoord::new(
                crate::map::HexCoord::new(0, 0),
                crate::map::HexCoord::new(0, 0),
            ),
            vertex: crate::map::surface_topology::types::SurfaceVertexId::new(0),
        });
    }

    Ok(())
}
