// src/map/height_graph/builder.rs
//! Pure combinatorial builder for Milestone M4.1 — Height Constraint Graph.

use crate::map::data::EdgeCoord;
use crate::map::height_constraints::types::HeightConstraintSet;
use crate::map::height_graph::diagnostics::*;
use crate::map::height_graph::types::*;
use crate::map::surface_topology::types::{
    SurfaceFaceId, SurfaceHalfEdgeId, SurfaceTopology, SurfaceVertexId,
};
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeightGraphBuildError {
    EmptySurfaceOnConstraints,
    PartialEmptySurface {
        vertex_count: usize,
        face_count: usize,
    },
    FaceNodeCountMismatch {
        expected: usize,
        actual: usize,
    },
    InvalidSurfaceFace(SurfaceFaceId),
    InvalidSurfaceVertex(SurfaceVertexId),
    InvalidSurfaceHalfEdge(SurfaceHalfEdgeId),
    MissingTwin(SurfaceHalfEdgeId),
    NonReciprocalTwin {
        a: SurfaceHalfEdgeId,
        b: SurfaceHalfEdgeId,
    },
    TwinOrientationMismatch {
        a: SurfaceHalfEdgeId,
        b: SurfaceHalfEdgeId,
    },
    FaceMissingVertex {
        face: SurfaceFaceId,
        vertex: SurfaceVertexId,
    },
    MixedSurfaceVerticesInNode {
        node: HeightNodeId,
    },
    MissingFaceCornerMapping {
        face: SurfaceFaceId,
        corner: u8,
    },
    DuplicateFaceCornerMapping {
        face: SurfaceFaceId,
        corner: u8,
    },
    RegionNodeMismatch {
        hex: HexCoord,
    },
    CliffRelationMismatch {
        edge: EdgeCoord,
        vertex: SurfaceVertexId,
    },
    InconsistentCliffVertexRelation {
        edge: EdgeCoord,
        vertex: SurfaceVertexId,
    },
    InvalidComponent(HeightSheetComponentId),
}

#[allow(dead_code)]
pub struct HeightGraphBuilderPlugin;

impl Plugin for HeightGraphBuilderPlugin {
    fn build(&self, _app: &mut App) {}
}

struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
        }
    }

    fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            i
        } else {
            let root = self.find(self.parent[i]);
            self.parent[i] = root;
            root
        }
    }

    fn union(&mut self, i: usize, j: usize) {
        let root_i = self.find(i);
        let root_j = self.find(j);
        if root_i != root_j {
            self.parent[root_i] = root_j;
        }
    }
}

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

    // 1. Defensive verification of M4 cliff boundary segments into canonical barrier set
    let mut barrier_half_edges = HashSet::new();
    for cliff in &constraints.cliffs {
        for seg in &cliff.segments {
            let a = surface.half_edges.get(seg.half_edge_a.index()).ok_or(
                HeightGraphBuildError::InvalidSurfaceHalfEdge(seg.half_edge_a),
            )?;
            let b = surface.half_edges.get(seg.half_edge_b.index()).ok_or(
                HeightGraphBuildError::InvalidSurfaceHalfEdge(seg.half_edge_b),
            )?;

            if a.twin != Some(seg.half_edge_b) || b.twin != Some(seg.half_edge_a) {
                return Err(HeightGraphBuildError::NonReciprocalTwin {
                    a: seg.half_edge_a,
                    b: seg.half_edge_b,
                });
            }

            if a.origin != b.destination || a.destination != b.origin {
                return Err(HeightGraphBuildError::TwinOrientationMismatch {
                    a: seg.half_edge_a,
                    b: seg.half_edge_b,
                });
            }

            let face_a = surface
                .faces
                .get(a.incident_face.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(a.incident_face))?;
            let face_b = surface
                .faces
                .get(b.incident_face.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(b.incident_face))?;

            if face_a.owner_hex != cliff.logical_edge.a || face_b.owner_hex != cliff.logical_edge.b
            {
                return Err(HeightGraphBuildError::CliffRelationMismatch {
                    edge: cliff.logical_edge,
                    vertex: a.origin,
                });
            }

            barrier_half_edges.insert(seg.half_edge_a);
            barrier_half_edges.insert(seg.half_edge_b);
        }
    }

    // 2. DSU over face-corner occurrences: occurrence_index = face_idx * 3 + corner
    let num_occurrences = surface.faces.len() * 3;
    let mut dsu = DisjointSet::new(num_occurrences);

    // 3. Union non-cliff reciprocal twin edges with total safe .get() lookups
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

        if twin.twin != Some(he_id) {
            return Err(HeightGraphBuildError::NonReciprocalTwin {
                a: he_id,
                b: twin_id,
            });
        }

        if half_edge.origin != twin.destination || half_edge.destination != twin.origin {
            return Err(HeightGraphBuildError::TwinOrientationMismatch {
                a: he_id,
                b: twin_id,
            });
        }

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

        let face_a_verts = face_a_ref.vertices;
        let face_b_verts = face_b_ref.vertices;

        let corner_a_u = face_a_verts.iter().position(|&v| v == u_vert).ok_or(
            HeightGraphBuildError::FaceMissingVertex {
                face: face_a_id,
                vertex: u_vert,
            },
        )?;
        let corner_b_u = face_b_verts.iter().position(|&v| v == u_vert).ok_or(
            HeightGraphBuildError::FaceMissingVertex {
                face: face_b_id,
                vertex: u_vert,
            },
        )?;

        dsu.union(
            face_a_id.index() * 3 + corner_a_u,
            face_b_id.index() * 3 + corner_b_u,
        );

        let corner_a_v = face_a_verts.iter().position(|&v| v == v_vert).ok_or(
            HeightGraphBuildError::FaceMissingVertex {
                face: face_a_id,
                vertex: v_vert,
            },
        )?;
        let corner_b_v = face_b_verts.iter().position(|&v| v == v_vert).ok_or(
            HeightGraphBuildError::FaceMissingVertex {
                face: face_b_id,
                vertex: v_vert,
            },
        )?;

        dsu.union(
            face_a_id.index() * 3 + corner_a_v,
            face_b_id.index() * 3 + corner_b_v,
        );
    }

    // 4. Collect equivalence classes
    let mut raw_classes: HashMap<usize, Vec<usize>> = HashMap::new();
    for occ_idx in 0..num_occurrences {
        let root = dsu.find(occ_idx);
        raw_classes.entry(root).or_default().push(occ_idx);
    }

    struct IntermediateNode {
        surface_vertex: SurfaceVertexId,
        incident_faces: Vec<SurfaceFaceId>,
        occurrences: Vec<usize>,
    }

    let mut intermediate_nodes = Vec::with_capacity(raw_classes.len());

    for (_root, occurrences) in raw_classes {
        let first_occ = occurrences[0];
        let first_face_idx = first_occ / 3;
        let first_corner = first_occ % 3;

        let first_face =
            surface
                .faces
                .get(first_face_idx)
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(
                    SurfaceFaceId::new(first_face_idx),
                ))?;
        let vertex_id = first_face.vertices[first_corner];

        let mut face_set = HashSet::new();

        for &occ_idx in &occurrences {
            let f_idx = occ_idx / 3;
            let corner = occ_idx % 3;
            let face =
                surface
                    .faces
                    .get(f_idx)
                    .ok_or(HeightGraphBuildError::InvalidSurfaceFace(
                        SurfaceFaceId::new(f_idx),
                    ))?;
            let v = face.vertices[corner];

            if v != vertex_id {
                return Err(HeightGraphBuildError::MixedSurfaceVerticesInNode {
                    node: HeightNodeId::new(0),
                });
            }

            face_set.insert(SurfaceFaceId::new(f_idx));
        }

        let mut incident_faces: Vec<_> = face_set.into_iter().collect();
        incident_faces.sort_by_key(|f| f.index());

        intermediate_nodes.push(IntermediateNode {
            surface_vertex: vertex_id,
            incident_faces,
            occurrences,
        });
    }

    // Deterministically sort equivalence classes by (surface_vertex.index(), min_incident_face.index())
    intermediate_nodes.sort_by_key(|n| (n.surface_vertex.index(), n.incident_faces[0].index()));

    let mut nodes = Vec::with_capacity(intermediate_nodes.len());
    let mut occurrence_to_node = vec![HeightNodeId::new(0); num_occurrences];

    for (node_idx, inter) in intermediate_nodes.into_iter().enumerate() {
        let node_id = HeightNodeId::new(node_idx);
        for occ_idx in inter.occurrences {
            occurrence_to_node[occ_idx] = node_id;
        }
        nodes.push(HeightNode {
            surface_vertex: inter.surface_vertex,
            incident_faces: inter.incident_faces,
        });
    }

    // 5. Build face_nodes table: 1:1 with surface.faces
    let mut face_nodes = Vec::with_capacity(surface.faces.len());
    for f_idx in 0..surface.faces.len() {
        let n0 = occurrence_to_node[f_idx * 3];
        let n1 = occurrence_to_node[f_idx * 3 + 1];
        let n2 = occurrence_to_node[f_idx * 3 + 2];
        face_nodes.push([n0, n1, n2]);
    }

    // 6. Build continuity_edges: (h0-h1, h1-h2, h2-h0) sorted & deduped
    let mut continuity_set = HashSet::new();
    for fn_arr in &face_nodes {
        continuity_set.insert(HeightContinuityEdge::new(fn_arr[0], fn_arr[1]));
        continuity_set.insert(HeightContinuityEdge::new(fn_arr[1], fn_arr[2]));
        continuity_set.insert(HeightContinuityEdge::new(fn_arr[2], fn_arr[0]));
    }
    let mut continuity_edges: Vec<_> = continuity_set.into_iter().collect();
    continuity_edges.sort();

    // 7. Compute connected components of HeightContinuityEdge
    let mut comp_dsu = DisjointSet::new(nodes.len());
    for edge in &continuity_edges {
        comp_dsu.union(edge.a.index(), edge.b.index());
    }

    let mut comp_map: HashMap<usize, Vec<HeightNodeId>> = HashMap::new();
    for n_idx in 0..nodes.len() {
        let root = comp_dsu.find(n_idx);
        comp_map
            .entry(root)
            .or_default()
            .push(HeightNodeId::new(n_idx));
    }

    let mut raw_components: Vec<_> = comp_map.into_values().collect();
    for comp_nodes in &mut raw_components {
        comp_nodes.sort_by_key(|n| n.index());
    }
    raw_components.sort_by_key(|c| c[0].index());

    let mut components = Vec::with_capacity(raw_components.len());
    let mut node_components = vec![HeightSheetComponentId::new(0); nodes.len()];

    for (comp_idx, comp_nodes) in raw_components.into_iter().enumerate() {
        let comp_id = HeightSheetComponentId::new(comp_idx);
        for &node in &comp_nodes {
            node_components[node.index()] = comp_id;
        }
        components.push(HeightSheetComponent { nodes: comp_nodes });
    }

    // 8. Region node constraints: region.faces -> face_nodes -> sort/dedup
    let mut regions = Vec::with_capacity(constraints.regions.len());
    for r in &constraints.regions {
        let mut r_nodes = HashSet::new();
        for &face in &r.faces {
            let fn_arr = face_nodes
                .get(face.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face))?;
            r_nodes.insert(fn_arr[0]);
            r_nodes.insert(fn_arr[1]);
            r_nodes.insert(fn_arr[2]);
        }
        let mut sorted_r_nodes: Vec<_> = r_nodes.into_iter().collect();
        sorted_r_nodes.sort_by_key(|n| n.index());
        regions.push(RegionNodeConstraint {
            hex: r.hex,
            intent: r.intent,
            nodes: sorted_r_nodes,
        });
    }

    // 9. Cliff node relations & deduplication
    let mut cliff_relations = Vec::new();
    let mut seen_cliff_samples: HashMap<
        (EdgeCoord, SurfaceVertexId),
        (HeightNodeId, HeightNodeId),
    > = HashMap::new();

    for cliff in &constraints.cliffs {
        for seg in &cliff.segments {
            let a = surface.half_edges.get(seg.half_edge_a.index()).ok_or(
                HeightGraphBuildError::InvalidSurfaceHalfEdge(seg.half_edge_a),
            )?;
            let b = surface.half_edges.get(seg.half_edge_b.index()).ok_or(
                HeightGraphBuildError::InvalidSurfaceHalfEdge(seg.half_edge_b),
            )?;

            let face_a_id = a.incident_face;
            let face_b_id = b.incident_face;

            let face_a_ref = surface
                .faces
                .get(face_a_id.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_a_id))?;
            let face_b_ref = surface
                .faces
                .get(face_b_id.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_b_id))?;

            // Sample endpoints: a.origin and a.destination
            for &vert_id in &[a.origin, a.destination] {
                let corner_a = face_a_ref
                    .vertices
                    .iter()
                    .position(|&v| v == vert_id)
                    .ok_or(HeightGraphBuildError::FaceMissingVertex {
                        face: face_a_id,
                        vertex: vert_id,
                    })?;
                let corner_b = face_b_ref
                    .vertices
                    .iter()
                    .position(|&v| v == vert_id)
                    .ok_or(HeightGraphBuildError::FaceMissingVertex {
                        face: face_b_id,
                        vertex: vert_id,
                    })?;

                let node_a = occurrence_to_node[face_a_id.index() * 3 + corner_a];
                let node_b = occurrence_to_node[face_b_id.index() * 3 + corner_b];

                let key = (cliff.logical_edge, vert_id);
                if let Some(&(prev_a, prev_b)) = seen_cliff_samples.get(&key) {
                    if prev_a != node_a || prev_b != node_b {
                        return Err(HeightGraphBuildError::InconsistentCliffVertexRelation {
                            edge: cliff.logical_edge,
                            vertex: vert_id,
                        });
                    }
                } else {
                    seen_cliff_samples.insert(key, (node_a, node_b));
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
    }

    cliff_relations.sort_by_key(|r| (r.logical_edge, r.surface_vertex.index()));

    // 10. Diagnostics generation
    let mut diagnostics = Vec::new();

    // Check unresolved and collapsed samples
    for rel in &cliff_relations {
        if rel.lower_side == crate::map::data::CliffLowerSide::Unresolved {
            diagnostics.push(HeightGraphDiagnostic {
                severity: HeightDiagnosticSeverity::Warning,
                kind: HeightGraphDiagnosticKind::UnresolvedCliff {
                    edge: rel.logical_edge,
                },
            });
        }
        if rel.node_a == rel.node_b {
            diagnostics.push(HeightGraphDiagnostic {
                severity: HeightDiagnosticSeverity::Info,
                kind: HeightGraphDiagnosticKind::CollapsedCliffSample {
                    edge: rel.logical_edge,
                    vertex: rel.surface_vertex,
                },
            });
        }
    }

    // Check fully unsplittable cliffs
    for cliff in &constraints.cliffs {
        let cliff_rels: Vec<_> = cliff_relations
            .iter()
            .filter(|r| r.logical_edge == cliff.logical_edge)
            .collect();
        if !cliff_rels.is_empty() && cliff_rels.iter().all(|r| r.node_a == r.node_b) {
            diagnostics.push(HeightGraphDiagnostic {
                severity: HeightDiagnosticSeverity::Error,
                kind: HeightGraphDiagnosticKind::UnsplittableCliff {
                    edge: cliff.logical_edge,
                },
            });
        }
    }

    // Symbolic ordering graph checks for non-collapsed, resolved relations
    let mut order_adj: HashMap<HeightNodeId, HashSet<HeightNodeId>> = HashMap::new();

    for rel in &cliff_relations {
        if rel.node_a == rel.node_b {
            continue;
        }
        if let Some((lower, higher)) = rel.resolved_order() {
            order_adj.entry(lower).or_default().insert(higher);
        }
    }

    // Check 2-node opposing ordering: lower -> higher AND higher -> lower
    let mut checked_pairs = HashSet::new();
    for (&u, neighbors) in &order_adj {
        for &v in neighbors {
            if u >= v {
                continue;
            }
            if let Some(rev_neighbors) = order_adj.get(&v) {
                if rev_neighbors.contains(&u) {
                    if checked_pairs.insert((u, v)) {
                        diagnostics.push(HeightGraphDiagnostic {
                            severity: HeightDiagnosticSeverity::Error,
                            kind: HeightGraphDiagnosticKind::OpposedCliffOrdering { a: u, b: v },
                        });
                    }
                }
            }
        }
    }

    // Check SCC cycles (>= 3 nodes) using Tarjan's algorithm
    let scc_list = find_sccs(&order_adj);
    for scc in scc_list {
        if scc.len() >= 3 {
            let mut sorted_scc = scc;
            sorted_scc.sort_by_key(|n| n.index());
            diagnostics.push(HeightGraphDiagnostic {
                severity: HeightDiagnosticSeverity::Error,
                kind: HeightGraphDiagnosticKind::DirectedCliffCycle {
                    component_nodes: sorted_scc,
                },
            });
        }
    }

    diagnostics.sort();
    diagnostics.dedup();

    let split_surface_vertex_count = {
        let mut vert_nodes: HashMap<SurfaceVertexId, HashSet<HeightNodeId>> = HashMap::new();
        for node in &nodes {
            vert_nodes
                .entry(node.surface_vertex)
                .or_default()
                .insert(HeightNodeId::new(0)); // count unique nodes per vertex
        }
        // Calculate count of SurfaceVertexIds that correspond to multiple HeightNodes
        let mut vert_node_counts: HashMap<SurfaceVertexId, usize> = HashMap::new();
        for (node_idx, node) in nodes.iter().enumerate() {
            *vert_node_counts.entry(node.surface_vertex).or_default() += 1;
        }
        vert_node_counts.values().filter(|&&cnt| cnt > 1).count()
    };

    let unresolved_cliff_count = cliff_relations
        .iter()
        .filter(|r| r.lower_side == crate::map::data::CliffLowerSide::Unresolved)
        .count();

    let error_diagnostic_count = diagnostics
        .iter()
        .filter(|d| d.severity == HeightDiagnosticSeverity::Error)
        .count();

    let stats = HeightGraphStats {
        node_count: nodes.len(),
        split_surface_vertex_count,
        continuity_edge_count: continuity_edges.len(),
        component_count: components.len(),
        region_constraint_count: regions.len(),
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
        regions,
        cliff_relations,
        diagnostics,
        stats,
    })
}

fn find_sccs(adj: &HashMap<HeightNodeId, HashSet<HeightNodeId>>) -> Vec<Vec<HeightNodeId>> {
    let mut index = 0usize;
    let mut stack = Vec::new();
    let mut indices = HashMap::new();
    let mut lowlink = HashMap::new();
    let mut on_stack = HashSet::new();
    let mut sccs = Vec::new();

    let all_nodes: HashSet<_> = adj
        .keys()
        .copied()
        .chain(adj.values().flatten().copied())
        .collect();

    for &node in &all_nodes {
        if !indices.contains_key(&node) {
            strongconnect(
                node,
                adj,
                &mut index,
                &mut stack,
                &mut indices,
                &mut lowlink,
                &mut on_stack,
                &mut sccs,
            );
        }
    }

    sccs
}

fn strongconnect(
    v: HeightNodeId,
    adj: &HashMap<HeightNodeId, HashSet<HeightNodeId>>,
    index: &mut usize,
    stack: &mut Vec<HeightNodeId>,
    indices: &mut HashMap<HeightNodeId, usize>,
    lowlink: &mut HashMap<HeightNodeId, usize>,
    on_stack: &mut HashSet<HeightNodeId>,
    sccs: &mut Vec<Vec<HeightNodeId>>,
) {
    indices.insert(v, *index);
    lowlink.insert(v, *index);
    *index += 1;
    stack.push(v);
    on_stack.insert(v);

    if let Some(neighbors) = adj.get(&v) {
        for &w in neighbors {
            if !indices.contains_key(&w) {
                strongconnect(w, adj, index, stack, indices, lowlink, on_stack, sccs);
                let w_low = lowlink[&w];
                let v_low = lowlink.get_mut(&v).unwrap();
                *v_low = (*v_low).min(w_low);
            } else if on_stack.contains(&w) {
                let w_index = indices[&w];
                let v_low = lowlink.get_mut(&v).unwrap();
                *v_low = (*v_low).min(w_index);
            }
        }
    }

    if lowlink[&v] == indices[&v] {
        let mut scc = Vec::new();
        loop {
            let w = stack.pop().unwrap();
            on_stack.remove(&w);
            scc.push(w);
            if w == v {
                break;
            }
        }
        sccs.push(scc);
    }
}
