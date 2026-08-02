//! Diagnostic shape-quality metrics for generated face topology.
use crate::map::face_topology::corner_key::regular_corner_position;
use crate::map::face_topology::types::HexFaceTopology;
use crate::map::face_topology::validation::signed_area;
use bevy::prelude::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TopologyMetrics {
    pub min_face_area: f32,
    pub max_face_area: f32,
    pub min_edge_length: f32,
    pub max_edge_length: f32,
    pub min_interior_angle: f32,
    pub max_interior_angle: f32,
    pub min_aspect_quality: f32,
    pub max_aspect_quality: f32,
    pub max_displacement: f32,
    pub average_displacement: f32,
}

fn face_points(topology: &HexFaceTopology, face_index: usize) -> [Vec2; 6] {
    let face = &topology.faces[face_index];
    std::array::from_fn(|index| topology.vertices[face.vertices[index].index()].position)
}

fn interior_angle(a: Vec2, b: Vec2, c: Vec2) -> f32 {
    let incoming = (a - b).normalize_or_zero();
    let outgoing = (c - b).normalize_or_zero();
    incoming.dot(outgoing).clamp(-1.0, 1.0).acos()
}

/// Computes non-gameplay shape metrics from the stored topology only.
#[must_use]
pub fn compute_topology_metrics(topology: &HexFaceTopology) -> TopologyMetrics {
    let mut metrics = TopologyMetrics {
        min_face_area: f32::INFINITY,
        max_face_area: f32::NEG_INFINITY,
        min_edge_length: f32::INFINITY,
        max_edge_length: f32::NEG_INFINITY,
        min_interior_angle: f32::INFINITY,
        max_interior_angle: f32::NEG_INFINITY,
        min_aspect_quality: f32::INFINITY,
        max_aspect_quality: f32::NEG_INFINITY,
        max_displacement: 0.0,
        average_displacement: 0.0,
    };
    for face_index in 0..topology.faces.len() {
        let points = face_points(topology, face_index);
        let area = signed_area(&points);
        let mut face_min_edge = f32::INFINITY;
        let mut face_max_edge: f32 = 0.0;
        for index in 0..6 {
            let edge_length = points[index].distance(points[(index + 1) % 6]);
            face_min_edge = face_min_edge.min(edge_length);
            face_max_edge = face_max_edge.max(edge_length);
            let angle = interior_angle(
                points[(index + 5) % 6],
                points[index],
                points[(index + 1) % 6],
            );
            metrics.min_interior_angle = metrics.min_interior_angle.min(angle);
            metrics.max_interior_angle = metrics.max_interior_angle.max(angle);
        }
        metrics.min_face_area = metrics.min_face_area.min(area);
        metrics.max_face_area = metrics.max_face_area.max(area);
        metrics.min_edge_length = metrics.min_edge_length.min(face_min_edge);
        metrics.max_edge_length = metrics.max_edge_length.max(face_max_edge);
        let aspect = face_min_edge / face_max_edge;
        metrics.min_aspect_quality = metrics.min_aspect_quality.min(aspect);
        metrics.max_aspect_quality = metrics.max_aspect_quality.max(aspect);
    }

    let mut displacement_sum = 0.0;
    for vertex in &topology.vertices {
        if let Ok(regular) = regular_corner_position(vertex.canonical_key) {
            let displacement = vertex.position.distance(regular);
            metrics.max_displacement = metrics.max_displacement.max(displacement);
            displacement_sum += displacement;
        }
    }
    if !topology.vertices.is_empty() {
        metrics.average_displacement = displacement_sum / topology.vertices.len() as f32;
    }
    metrics
}
