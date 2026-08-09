// src/map/height_graph/builder_dsu.rs
//! DSU occurrence partitioning and equivalence class logic for `HeightConstraintGraph`.

use crate::map::height_constraints::types::HeightConstraintSet;
use crate::map::height_graph::types::HeightGraphBuildError;
use crate::map::height_graph::types::{HeightNode, HeightNodeId};
use crate::map::surface_topology::types::{
    SurfaceFaceId, SurfaceHalfEdgeId, SurfaceTopology, SurfaceVertexId,
};
use bevy::prelude::*;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct HeightGraphBuilderDsuPlugin;

impl Plugin for HeightGraphBuilderDsuPlugin {
    fn build(&self, _app: &mut App) {}
}

pub struct DisjointSet {
    parent: Vec<usize>,
}

impl DisjointSet {
    #[must_use]
    pub fn new(size: usize) -> Self {
        Self {
            parent: (0..size).collect(),
        }
    }

    pub fn find(&mut self, i: usize) -> usize {
        if self.parent[i] == i {
            i
        } else {
            let root = self.find(self.parent[i]);
            self.parent[i] = root;
            root
        }
    }

    pub fn union(&mut self, i: usize, j: usize) {
        let root_i = self.find(i);
        let root_j = self.find(j);
        if root_i != root_j {
            self.parent[root_i] = root_j;
        }
    }
}

pub struct IntermediateNode {
    pub surface_vertex: SurfaceVertexId,
    pub incident_faces: Vec<SurfaceFaceId>,
    pub occurrences: Vec<usize>,
}

/// Builds height nodes via DSU occurrence partitioning.
///
/// # Errors
/// Returns `HeightGraphBuildError` if half-edge or face topology bounds are invalid.
#[allow(
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::missing_errors_doc,
    clippy::cast_possible_truncation
)]
pub fn build_height_nodes_via_dsu(
    surface: &SurfaceTopology,
    constraints: &HeightConstraintSet,
) -> Result<
    (
        Vec<HeightNode>,
        Vec<[HeightNodeId; 3]>,
        HashMap<(SurfaceFaceId, u8), HeightNodeId>,
    ),
    HeightGraphBuildError,
> {
    let num_occurrences = surface.faces.len() * 3;
    let mut dsu = DisjointSet::new(num_occurrences);

    let mut cliff_seam_halfs: HashMap<SurfaceHalfEdgeId, SurfaceHalfEdgeId> = HashMap::new();
    for cliff in &constraints.cliffs {
        for segment in &cliff.segments {
            cliff_seam_halfs.insert(segment.half_edge_a, segment.half_edge_b);
            cliff_seam_halfs.insert(segment.half_edge_b, segment.half_edge_a);
        }
    }

    for (face_a_idx, face_a) in surface.faces.iter().enumerate() {
        let face_a_id = SurfaceFaceId::new(face_a_idx);
        let he_start = face_a.boundary;

        let mut current_he_id = he_start;
        for _corner_idx in 0..3 {
            let he = surface
                .half_edges
                .get(current_he_id.index())
                .ok_or(HeightGraphBuildError::InvalidSurfaceHalfEdge(current_he_id))?;

            if !cliff_seam_halfs.contains_key(&current_he_id) {
                if let Some(twin_id) = he.twin {
                    let twin = surface
                        .half_edges
                        .get(twin_id.index())
                        .ok_or(HeightGraphBuildError::InvalidSurfaceHalfEdge(twin_id))?;

                    if twin.twin != Some(current_he_id) {
                        return Err(HeightGraphBuildError::NonReciprocalTwin {
                            a: current_he_id,
                            b: twin_id,
                        });
                    }

                    let u_vert = he.origin;
                    let v_vert = he.destination;

                    if twin.origin != v_vert || twin.destination != u_vert {
                        return Err(HeightGraphBuildError::TwinOrientationMismatch {
                            a: current_he_id,
                            b: twin_id,
                        });
                    }

                    let face_b_id = twin.incident_face;
                    let face_b = surface
                        .faces
                        .get(face_b_id.index())
                        .ok_or(HeightGraphBuildError::InvalidSurfaceFace(face_b_id))?;

                    let face_a_verts = face_a.vertices;
                    let face_b_verts = face_b.vertices;

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
            }

            current_he_id = he.next;
        }
    }

    let mut raw_classes: HashMap<usize, Vec<usize>> = HashMap::new();
    for occ_idx in 0..num_occurrences {
        let root = dsu.find(occ_idx);
        raw_classes.entry(root).or_default().push(occ_idx);
    }

    let mut intermediate_nodes = Vec::with_capacity(raw_classes.len());

    for (_root, occurrences) in raw_classes {
        let first_occ = occurrences[0];
        let face_idx = first_occ / 3;
        let corner_idx = first_occ % 3;
        let face = surface
            .faces
            .get(face_idx)
            .ok_or(HeightGraphBuildError::InvalidSurfaceFace(
                SurfaceFaceId::new(face_idx),
            ))?;
        let surface_vertex = face.vertices[corner_idx];

        let mut incident_faces = Vec::new();
        for &occ in &occurrences {
            let f_idx = occ / 3;
            let f_id = SurfaceFaceId::new(f_idx);
            if !incident_faces.contains(&f_id) {
                incident_faces.push(f_id);
            }
        }
        incident_faces.sort();

        intermediate_nodes.push(IntermediateNode {
            surface_vertex,
            incident_faces,
            occurrences,
        });
    }

    intermediate_nodes.sort_by(|a, b| {
        a.surface_vertex
            .cmp(&b.surface_vertex)
            .then_with(|| a.incident_faces.cmp(&b.incident_faces))
    });

    let mut nodes = Vec::with_capacity(intermediate_nodes.len());
    let mut occ_to_node: HashMap<usize, HeightNodeId> = HashMap::new();

    for (node_idx, inter) in intermediate_nodes.into_iter().enumerate() {
        let node_id = HeightNodeId::new(node_idx);
        for &occ in &inter.occurrences {
            occ_to_node.insert(occ, node_id);
        }
        nodes.push(HeightNode {
            surface_vertex: inter.surface_vertex,
            incident_faces: inter.incident_faces,
        });
    }

    let mut face_nodes = Vec::with_capacity(surface.faces.len());
    let mut face_corner_to_node = HashMap::new();

    for (face_idx, _face) in surface.faces.iter().enumerate() {
        let face_id = SurfaceFaceId::new(face_idx);
        let base_occ = face_id.index() * 3;

        let n0 = occ_to_node.get(&(base_occ)).copied().ok_or(
            HeightGraphBuildError::MissingFaceCornerMapping {
                face: face_id,
                corner: 0,
            },
        )?;
        let n1 = occ_to_node.get(&(base_occ + 1)).copied().ok_or(
            HeightGraphBuildError::MissingFaceCornerMapping {
                face: face_id,
                corner: 1,
            },
        )?;
        let n2 = occ_to_node.get(&(base_occ + 2)).copied().ok_or(
            HeightGraphBuildError::MissingFaceCornerMapping {
                face: face_id,
                corner: 2,
            },
        )?;

        let k0 = (face_id, 0);
        let k1 = (face_id, 1);
        let k2 = (face_id, 2);
        face_corner_to_node.insert(k0, n0);
        face_corner_to_node.insert(k1, n1);
        face_corner_to_node.insert(k2, n2);

        face_nodes.push([n0, n1, n2]);
    }

    Ok((nodes, face_nodes, face_corner_to_node))
}
