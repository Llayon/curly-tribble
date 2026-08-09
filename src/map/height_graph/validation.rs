// src/map/height_graph/validation.rs
//! Structural validator and partition completeness proofs for Milestone M4.1 — Height Constraint Graph.

use crate::map::height_constraints::types::HeightConstraintSet;
use crate::map::height_graph::builder::HeightGraphBuildError;
use crate::map::height_graph::types::HeightConstraintGraph;
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceHalfEdgeId, SurfaceTopology};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[allow(dead_code)]
pub struct HeightGraphValidationPlugin;

impl Plugin for HeightGraphValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn validate_height_constraint_graph(
    graph: &HeightConstraintGraph,
    surface: &SurfaceTopology,
    constraints: &HeightConstraintSet,
) -> Result<(), HeightGraphBuildError> {
    // 0. Prioritized symmetrical empty surface validation
    let vertices_empty = surface.vertices.is_empty();
    let faces_empty = surface.faces.is_empty();

    match (vertices_empty, faces_empty) {
        (true, false) | (false, true) => {
            return Err(HeightGraphBuildError::PartialEmptySurface {
                vertex_count: surface.vertices.len(),
                face_count: surface.faces.len(),
            });
        }
        (true, true) => {
            if graph.nodes.is_empty()
                && graph.face_nodes.is_empty()
                && constraints.regions.is_empty()
                && constraints.cliffs.is_empty()
            {
                return Ok(());
            }
            return Err(HeightGraphBuildError::EmptySurfaceOnConstraints);
        }
        (false, false) => {}
    }

    // 1. Verify face_nodes length == surface.faces.len()
    if graph.face_nodes.len() != surface.faces.len() {
        return Err(HeightGraphBuildError::FaceNodeCountMismatch {
            expected: surface.faces.len(),
            actual: graph.face_nodes.len(),
        });
    }

    // 2. Occurrence Partition Proof
    let mut mapped_occurrences = 0;
    let mut seen_corners: HashSet<(SurfaceFaceId, u8)> = HashSet::new();

    for (node_idx, node) in graph.nodes.iter().enumerate() {
        let node_id = crate::map::height_graph::types::HeightNodeId::new(node_idx);

        if node.incident_faces.is_empty() {
            return Err(HeightGraphBuildError::InvalidSurfaceFace(
                SurfaceFaceId::new(0),
            ));
        }

        for &face_id in &node.incident_faces {
            let face = surface
                .faces
                .get(face_id.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_id))?;

            for (corner_idx, &v_id) in face.vertices.iter().enumerate() {
                if v_id == node.surface_vertex {
                    let fn_arr = graph
                        .face_nodes
                        .get(face_id.index())
                        .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_id))?;

                    if fn_arr[corner_idx] != node_id {
                        return Err(HeightGraphBuildError::MissingFaceCornerMapping {
                            face: face_id,
                            corner: corner_idx as u8,
                        });
                    }

                    if !seen_corners.insert((face_id, corner_idx as u8)) {
                        return Err(HeightGraphBuildError::DuplicateFaceCornerMapping {
                            face: face_id,
                            corner: corner_idx as u8,
                        });
                    }

                    mapped_occurrences += 1;
                }
            }
        }
    }

    if mapped_occurrences != surface.faces.len() * 3 {
        return Err(HeightGraphBuildError::FaceNodeCountMismatch {
            expected: surface.faces.len() * 3,
            actual: mapped_occurrences,
        });
    }

    // 3. Continuity / Cut Proof
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

    // 4. Region Completeness Proof
    for r in &constraints.regions {
        let mut expected_nodes = HashSet::new();
        for &face in &r.faces {
            let fn_arr = graph
                .face_nodes
                .get(face.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face))?;
            expected_nodes.insert(fn_arr[0]);
            expected_nodes.insert(fn_arr[1]);
            expected_nodes.insert(fn_arr[2]);
        }
        let mut expected_sorted: Vec<_> = expected_nodes.into_iter().collect();
        expected_sorted.sort_by_key(|n| n.index());

        let actual = graph
            .region_nodes_for_hex(r.hex)
            .ok_or(HeightGraphBuildError::RegionNodeMismatch { hex: r.hex })?;

        if actual != expected_sorted.as_slice() {
            return Err(HeightGraphBuildError::RegionNodeMismatch { hex: r.hex });
        }
    }

    // 5. Cliff Relation Completeness Proof
    for cliff in &constraints.cliffs {
        for seg in &cliff.segments {
            let a = surface.half_edges.get(seg.half_edge_a.index()).ok_or(
                HeightGraphBuildError::InvalidSurfaceHalfEdge(seg.half_edge_a),
            )?;

            for &vert_id in &[a.origin, a.destination] {
                let found = graph
                    .cliff_relations_for_edge(cliff.logical_edge)
                    .any(|rel| rel.surface_vertex == vert_id && rel.lower_side == cliff.lower_side);

                if !found {
                    return Err(HeightGraphBuildError::CliffRelationMismatch {
                        edge: cliff.logical_edge,
                        vertex: vert_id,
                    });
                }
            }
        }
    }

    Ok(())
}
