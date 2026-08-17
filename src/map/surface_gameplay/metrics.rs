// src/map/surface_gameplay/metrics.rs
//! Milestone M6 — pure derivation of `SurfaceMetricField` from the solved
//! surface (`SurfaceTopology` + `SurfaceTerrainBake`).
//!
//! No `MapData`, `TileData`, `TerrainType`, `OceanState`, `MAX_HEIGHT`,
//! `HEX_SIZE`, `Mesh`, `TerrainTopology`, `TerrainConfig`,
//! `compute_vertex_heights`, or `.elevation` (Guard §30).

use crate::map::surface_gameplay::edges::collect_edge_metrics;
use crate::map::surface_gameplay::types::{
    HexSurfaceMetrics, SurfaceMetricField, SurfaceMetricFieldStats, SurfaceMetricsError,
};
use crate::map::surface_topology::types::{SurfaceTopology, SurfaceVertexId};
use crate::map::terrain_bake::types::SurfaceTerrainBake;
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

pub struct SurfaceGameplayMetricsPlugin;

impl Plugin for SurfaceGameplayMetricsPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Derives the pure-geometry metric layer from the solved surface.
///
/// # Errors
/// Returns `SurfaceMetricsError` on missing/duplicate centers, partial-empty
/// inputs, or invalid surface references.
pub fn derive_surface_metrics(
    surface: &SurfaceTopology,
    bake: &SurfaceTerrainBake,
) -> Result<SurfaceMetricField, SurfaceMetricsError> {
    let all_empty = surface.vertices.is_empty()
        && surface.faces.is_empty()
        && bake.vertices.is_empty()
        && bake.faces.is_empty();
    if all_empty {
        return Ok(SurfaceMetricField::default());
    }

    if !surface.vertices.is_empty()
        && (surface.faces.is_empty() || bake.vertices.is_empty() || bake.faces.is_empty())
    {
        return Err(SurfaceMetricsError::PartialEmptyInputs {
            surface_vertices: surface.vertices.len(),
            surface_faces: surface.faces.len(),
            bake_vertices: bake.vertices.len(),
            bake_faces: bake.faces.len(),
        });
    }

    let mut cells = BTreeMap::new();
    for (hex, center_vertex) in collect_hex_centers(surface)? {
        let metrics = derive_hex_metrics(surface, bake, hex, center_vertex)?;
        cells.insert(hex, metrics);
    }

    let (mut edges, seam_edges) = collect_edge_metrics(surface, bake)?;
    for edge in &seam_edges {
        if let Some(metrics) = edges.get_mut(edge) {
            metrics.height_seam = true;
        }
    }

    let stats = SurfaceMetricFieldStats {
        cell_count: cells.len(),
        edge_count: edges.len(),
        seam_edge_count: seam_edges.len(),
    };

    Ok(SurfaceMetricField {
        cells,
        edges,
        stats,
    })
}

// ─── Hex centers ──────────────────────────────────────────────────────────────

/// Maps every hex to its exact center vertex (`SurfaceVertexSource::HexCenter`).
///
/// # Errors
/// Returns `SurfaceMetricsError` when a hex has 0 or 2+ center vertices.
fn collect_hex_centers(
    surface: &SurfaceTopology,
) -> Result<BTreeMap<HexCoord, SurfaceVertexId>, SurfaceMetricsError> {
    use crate::map::surface_topology::types::SurfaceVertexSource;

    let mut centers: BTreeMap<HexCoord, SurfaceVertexId> = BTreeMap::new();
    for (idx, vertex) in surface.vertices.iter().enumerate() {
        if let SurfaceVertexSource::HexCenter { hex } = vertex.source {
            let vertex_id = SurfaceVertexId::new(idx);
            if centers.insert(hex, vertex_id).is_some() {
                return Err(SurfaceMetricsError::DuplicateHexCenter(hex));
            }
        }
    }

    let known_hexes: BTreeSet<HexCoord> = surface.faces.iter().map(|f| f.owner_hex).collect();
    for hex in known_hexes {
        if !centers.contains_key(&hex) {
            return Err(SurfaceMetricsError::MissingHexCenter(hex));
        }
    }

    Ok(centers)
}

// ─── Per-hex metrics ──────────────────────────────────────────────────────────

/// Computes center height, relief, and max internal step for one hex.
///
/// # Errors
/// Returns `SurfaceMetricsError` on missing/ambiguous center node or
/// non-finite center height.
fn derive_hex_metrics(
    surface: &SurfaceTopology,
    bake: &SurfaceTerrainBake,
    hex: HexCoord,
    center_vertex: SurfaceVertexId,
) -> Result<HexSurfaceMetrics, SurfaceMetricsError> {
    let center_xz = surface
        .vertices
        .get(center_vertex.index())
        .map(|v| v.position)
        .ok_or(SurfaceMetricsError::InvalidSurfaceVertex(center_vertex))?;

    let center_nodes: Vec<_> = bake
        .vertices
        .iter()
        .filter(|v| v.surface_vertex == center_vertex)
        .map(|v| v.height_node)
        .collect();

    let center_node = match center_nodes.as_slice() {
        [] => return Err(SurfaceMetricsError::MissingCenterHeightNode(hex)),
        [node] => *node,
        _ => return Err(SurfaceMetricsError::AmbiguousCenterHeightNode(hex)),
    };

    let center_height = bake
        .vertices
        .get(center_node.index())
        .map(|v| v.normalized_height)
        .ok_or(SurfaceMetricsError::MissingCenterHeightNode(hex))?;
    if !center_height.is_finite() {
        return Err(SurfaceMetricsError::NonFiniteHeight(hex));
    }

    let owned_heights: Vec<f32> = bake
        .vertices
        .iter()
        .filter(|v| v.owner_hexes.contains(&hex))
        .map(|v| v.normalized_height)
        .collect();

    let relief = compute_relief(&owned_heights);

    let mut internal_step = 0.0f32;
    for face in bake.faces.iter().filter(|f| f.owner_hex == hex) {
        let [a, b, c] = face.nodes;
        for (n0, n1) in [(a, b), (b, c), (c, a)] {
            let h0 = bake.vertices.get(n0.index()).map(|v| v.normalized_height);
            let h1 = bake.vertices.get(n1.index()).map(|v| v.normalized_height);
            if let (Some(h0), Some(h1)) = (h0, h1) {
                internal_step = internal_step.max((h0 - h1).abs());
            }
        }
    }

    Ok(HexSurfaceMetrics {
        center_xz,
        center_height,
        relief,
        max_internal_step: internal_step,
    })
}

/// `max - min` of owned node heights; `0.0` when empty.
fn compute_relief(heights: &[f32]) -> f32 {
    let mut min_h = f32::INFINITY;
    let mut max_h = f32::NEG_INFINITY;
    for &h in heights {
        min_h = min_h.min(h);
        max_h = max_h.max(h);
    }
    if min_h.is_finite() && max_h.is_finite() {
        max_h - min_h
    } else {
        0.0
    }
}
