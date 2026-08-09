// src/map/height_graph/builder.rs
//! Pure combinatorial builder for Milestone M4.1 — Height Constraint Graph.

use crate::map::data::CliffLowerSide;
use crate::map::height_constraints::types::HeightConstraintSet;
use crate::map::height_graph::builder_diagnostics::collect_height_graph_diagnostics;
use crate::map::height_graph::builder_dsu::build_height_nodes_via_dsu;
use crate::map::height_graph::types::{
    CliffNodeRelation, HeightConstraintGraph, HeightContinuityEdge, HeightGraphBuildError,
    HeightGraphStats, HeightNodeId, HeightSheetComponent, HeightSheetComponentId,
    RegionNodeConstraint,
};
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceTopology, SurfaceVertexId};
use bevy::prelude::*;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct HeightGraphBuilderPlugin;

impl Plugin for HeightGraphBuilderPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Builds derived `HeightConstraintGraph` from `SurfaceTopology` and `HeightConstraintSet`.
///
/// # Errors
/// Returns `HeightGraphBuildError` if topological bounds or surface consistency is violated.
#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn build_height_constraint_graph(
    surface: &SurfaceTopology,
    constraints: &HeightConstraintSet,
) -> Result<HeightConstraintGraph, HeightGraphBuildError> {
    // 0. Prioritized symmetrical surface validation
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
            if constraints.regions.is_empty() && constraints.cliffs.is_empty() {
                return Ok(HeightConstraintGraph::default());
            }
            return Err(HeightGraphBuildError::EmptySurfaceOnConstraints);
        }
        (false, false) => {}
    }

    // 1-4. DSU occurrence partitioning & node creation
    let (nodes, face_nodes, face_corner_to_node) =
        build_height_nodes_via_dsu(surface, constraints)?;

    // 5. Build height continuity edges
    let mut continuity_edges = Vec::new();

    for (face_idx, _face) in surface.faces.iter().enumerate() {
        let face_id = SurfaceFaceId::new(face_idx);
        for corner_idx in 0..3 {
            let next_corner = (corner_idx + 1) % 3;
            let n_from = *face_corner_to_node
                .get(&(face_id, corner_idx as u8))
                .ok_or(HeightGraphBuildError::MissingFaceCornerMapping {
                    face: face_id,
                    corner: corner_idx as u8,
                })?;
            let n_to = *face_corner_to_node
                .get(&(face_id, next_corner as u8))
                .ok_or(HeightGraphBuildError::MissingFaceCornerMapping {
                    face: face_id,
                    corner: next_corner as u8,
                })?;

            if n_from != n_to {
                continuity_edges.push(HeightContinuityEdge::new(n_from, n_to));
            }
        }
    }
    continuity_edges.sort();
    continuity_edges.dedup();

    // 6. Connected components over continuity edges
    let mut comp_dsu = crate::map::height_graph::builder_dsu::DisjointSet::new(nodes.len());
    for edge in &continuity_edges {
        comp_dsu.union(edge.a.index(), edge.b.index());
    }

    let mut comp_classes: HashMap<usize, Vec<HeightNodeId>> = HashMap::new();
    for (node_idx, _node) in nodes.iter().enumerate() {
        let node_id = HeightNodeId::new(node_idx);
        let root = comp_dsu.find(node_idx);
        comp_classes.entry(root).or_default().push(node_id);
    }

    let mut components_raw: Vec<Vec<HeightNodeId>> = comp_classes.into_values().collect();
    for comp in &mut components_raw {
        comp.sort();
    }
    components_raw.sort();

    let mut components = Vec::with_capacity(components_raw.len());
    let mut node_to_comp = HashMap::new();

    for (comp_idx, nodes_in_comp) in components_raw.into_iter().enumerate() {
        let comp_id = HeightSheetComponentId::new(comp_idx);
        for &node_id in &nodes_in_comp {
            node_to_comp.insert(node_id, comp_id);
        }
        components.push(HeightSheetComponent {
            nodes: nodes_in_comp,
        });
    }

    let mut node_components = Vec::with_capacity(nodes.len());
    for node_idx in 0..nodes.len() {
        let node_id = HeightNodeId::new(node_idx);
        let comp_id =
            node_to_comp
                .get(&node_id)
                .copied()
                .ok_or(HeightGraphBuildError::InvalidComponent(
                    HeightSheetComponentId::new(0),
                ))?;
        node_components.push(comp_id);
    }

    // 7. Bind Region Node Constraints
    let mut region_node_constraints = Vec::with_capacity(constraints.regions.len());
    for region in &constraints.regions {
        let mut bound_nodes = Vec::new();
        for &f_id in &region.faces {
            let f_nodes = face_nodes
                .get(f_id.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(f_id))?;
            for &n_id in f_nodes {
                if !bound_nodes.contains(&n_id) {
                    bound_nodes.push(n_id);
                }
            }
        }
        bound_nodes.sort();
        region_node_constraints.push(RegionNodeConstraint {
            hex: region.hex,
            intent: region.intent,
            nodes: bound_nodes,
        });
    }
    region_node_constraints.sort_by_key(|r| r.hex);

    // 8. Bind Cliff Node Relations
    let mut cliff_relations = Vec::new();
    for cliff in &constraints.cliffs {
        for segment in &cliff.segments {
            let he_a = surface.half_edges.get(segment.half_edge_a.index()).ok_or(
                HeightGraphBuildError::InvalidSurfaceHalfEdge(segment.half_edge_a),
            )?;
            let he_b = surface.half_edges.get(segment.half_edge_b.index()).ok_or(
                HeightGraphBuildError::InvalidSurfaceHalfEdge(segment.half_edge_b),
            )?;

            let face_a_id = he_a.incident_face;
            let face_b_id = he_b.incident_face;

            let face_a = surface
                .faces
                .get(face_a_id.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_a_id))?;
            let face_b = surface
                .faces
                .get(face_b_id.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_b_id))?;

            for &vert_id in &[he_a.origin, he_a.destination] {
                let corner_a_v = face_a.vertices.iter().position(|&v| v == vert_id).ok_or(
                    HeightGraphBuildError::FaceMissingVertex {
                        face: face_a_id,
                        vertex: vert_id,
                    },
                )?;
                let corner_b_v = face_b.vertices.iter().position(|&v| v == vert_id).ok_or(
                    HeightGraphBuildError::FaceMissingVertex {
                        face: face_b_id,
                        vertex: vert_id,
                    },
                )?;

                let node_a = *face_corner_to_node
                    .get(&(face_a_id, corner_a_v as u8))
                    .ok_or(HeightGraphBuildError::MissingFaceCornerMapping {
                        face: face_a_id,
                        corner: corner_a_v as u8,
                    })?;
                let node_b = *face_corner_to_node
                    .get(&(face_b_id, corner_b_v as u8))
                    .ok_or(HeightGraphBuildError::MissingFaceCornerMapping {
                        face: face_b_id,
                        corner: corner_b_v as u8,
                    })?;

                cliff_relations.push(CliffNodeRelation {
                    logical_edge: cliff.logical_edge,
                    surface_vertex: vert_id,
                    node_a,
                    node_b,
                    lower_side: cliff.lower_side,
                });
            }
        }
    }
    cliff_relations.sort_by(|a, b| {
        a.logical_edge
            .cmp(&b.logical_edge)
            .then_with(|| a.surface_vertex.cmp(&b.surface_vertex))
            .then_with(|| a.node_a.cmp(&b.node_a))
            .then_with(|| a.node_b.cmp(&b.node_b))
    });
    cliff_relations.dedup();

    // 9. Collect diagnostics
    let diagnostics = collect_height_graph_diagnostics(&cliff_relations);

    // 10. Compute stats
    let split_surface_vertex_count = compute_split_surface_vertex_count(&nodes);
    let error_diagnostic_count = diagnostics
        .iter()
        .filter(|d| {
            d.severity == crate::map::height_graph::diagnostics::HeightDiagnosticSeverity::Error
        })
        .count();
    let unresolved_cliff_count = cliff_relations
        .iter()
        .filter(|r| r.lower_side == CliffLowerSide::Unresolved)
        .count();

    let stats = HeightGraphStats {
        node_count: nodes.len(),
        split_surface_vertex_count,
        continuity_edge_count: continuity_edges.len(),
        component_count: components.len(),
        region_constraint_count: region_node_constraints.len(),
        cliff_relation_count: cliff_relations.len(),
        unresolved_cliff_count,
        diagnostic_count: diagnostics.len(),
        error_diagnostic_count,
    };

    Ok(HeightConstraintGraph {
        nodes,
        face_nodes,
        continuity_edges,
        node_components,
        components,
        regions: region_node_constraints,
        cliff_relations,
        diagnostics,
        stats,
    })
}

fn compute_split_surface_vertex_count(
    nodes: &[crate::map::height_graph::types::HeightNode],
) -> usize {
    let mut vert_node_counts: HashMap<SurfaceVertexId, usize> = HashMap::new();
    for node in nodes {
        *vert_node_counts.entry(node.surface_vertex).or_default() += 1;
    }
    vert_node_counts.values().filter(|&&cnt| cnt > 1).count()
}
