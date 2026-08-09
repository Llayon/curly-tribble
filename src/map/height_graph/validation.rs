// src/map/height_graph/validation.rs
//! Structural validator for Milestone M4.1 — Height Constraint Graph.

use crate::map::height_constraints::types::HeightConstraintSet;
use crate::map::height_graph::types::{HeightConstraintGraph, HeightGraphBuildError};
use crate::map::height_graph::validation_completeness::{
    validate_cliff_relations_completeness, validate_non_cliff_continuity,
};
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceTopology};
use bevy::prelude::*;
use std::collections::HashSet;

#[allow(dead_code)]
pub struct HeightGraphValidationPlugin;

impl Plugin for HeightGraphValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Validates derived `HeightConstraintGraph` structural partition and completeness.
///
/// # Errors
/// Returns `HeightGraphBuildError` if structural partition completeness or continuity is broken.
#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
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
    validate_occurrence_partition(graph, surface)?;

    // 3. Continuity / Cut Proof
    validate_non_cliff_continuity(graph, surface, constraints)?;

    // 4. Region Completeness Proof
    validate_region_completeness(graph, constraints)?;

    // 5. Cliff Relation Completeness Proof
    validate_cliff_relations_completeness(graph, surface, constraints)?;

    Ok(())
}

#[allow(clippy::cast_possible_truncation, clippy::missing_errors_doc)]
fn validate_occurrence_partition(
    graph: &HeightConstraintGraph,
    surface: &SurfaceTopology,
) -> Result<(), HeightGraphBuildError> {
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

                    let corner_key = (face_id, corner_idx as u8);
                    if !seen_corners.insert(corner_key) {
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

    Ok(())
}

fn validate_region_completeness(
    graph: &HeightConstraintGraph,
    constraints: &HeightConstraintSet,
) -> Result<(), HeightGraphBuildError> {
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

    Ok(())
}
