//! Pure cached data and identity-based edge extraction for diagnostics.
use crate::map::face_topology::corner_key::canonical_corner_key;
use crate::map::face_topology::types::{HexFaceTopology, SharedCornerKey, VertexId};
use crate::map::MapData;
use bevy::prelude::Resource;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UniqueUndirectedEdge {
    pub min: VertexId,
    pub max: VertexId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UniqueRegularEdge {
    pub min: SharedCornerKey,
    pub max: SharedCornerKey,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct HexFaceDebugCache {
    pub edges: Vec<UniqueUndirectedEdge>,
    pub regular_edges: Vec<UniqueRegularEdge>,
    pub shared_vertices: Vec<VertexId>,
}

impl HexFaceDebugCache {
    pub fn rebuild(&mut self, topology: &HexFaceTopology, map_data: &MapData) {
        self.edges = extract_unique_undirected_edges(topology);
        self.regular_edges = extract_unique_regular_edges(map_data);
        self.shared_vertices = extract_shared_vertices(topology);
    }

    pub fn clear(&mut self) {
        self.edges.clear();
        self.regular_edges.clear();
        self.shared_vertices.clear();
    }

    #[must_use]
    pub fn is_consistent(&self, topology: &HexFaceTopology) -> bool {
        let expected_edges = topology.stats.paired_edge_count + topology.stats.border_edge_count;
        let warped_unique: HashSet<_> = self.edges.iter().copied().collect();
        let regular_unique: HashSet<_> = self.regular_edges.iter().copied().collect();
        self.edges.len() == expected_edges
            && self.regular_edges.len() == expected_edges
            && self.edges.len() == self.regular_edges.len()
            && self.shared_vertices.len() == topology.vertices.len()
            && warped_unique.len() == self.edges.len()
            && regular_unique.len() == self.regular_edges.len()
    }
}

/// Extracts each topology edge once using `VertexId` identity, never positions.
#[must_use]
pub fn extract_unique_undirected_edges(topology: &HexFaceTopology) -> Vec<UniqueUndirectedEdge> {
    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    for edge in &topology.half_edges {
        let (min, max) = if edge.origin <= edge.destination {
            (edge.origin, edge.destination)
        } else {
            (edge.destination, edge.origin)
        };
        let unique_edge = UniqueUndirectedEdge { min, max };
        if seen.insert(unique_edge) {
            edges.push(unique_edge);
        }
    }
    edges
}

/// Extracts each regular logical hex edge once using canonical corner keys.
#[must_use]
pub fn extract_unique_regular_edges(map_data: &MapData) -> Vec<UniqueRegularEdge> {
    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    for &coord in map_data.tiles.keys() {
        for index in 0..6 {
            let corner_a = canonical_corner_key(coord, index);
            let corner_b = canonical_corner_key(coord, (index + 1) % 6);
            let (min, max) = if corner_a <= corner_b {
                (corner_a, corner_b)
            } else {
                (corner_b, corner_a)
            };
            let edge = UniqueRegularEdge { min, max };
            if seen.insert(edge) {
                edges.push(edge);
            }
        }
    }
    edges.sort_unstable();
    edges
}

/// Returns one marker identity per canonical stored `MapVertex`.
#[must_use]
pub fn extract_shared_vertices(topology: &HexFaceTopology) -> Vec<VertexId> {
    (0..topology.vertices.len()).map(VertexId::new).collect()
}
