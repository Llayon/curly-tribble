// src/map/surface_gameplay/compiler.rs
//! Milestone M6 — stage 2: compile the authoritative `SurfaceGameplayMap`
//! from pure-geometry metrics + legacy `MapData` *classification* fields
//! (terrain / ocean state only — never `TileData.elevation`).

use crate::map::data::{EdgeCoord, MapData, OceanState, TerrainType};
use crate::map::surface_gameplay::config::SurfaceGameplayConfig;
use crate::map::surface_gameplay::types::{
    SurfaceGameplayCell, SurfaceGameplayCompileError, SurfaceGameplayEdge, SurfaceGameplayMap,
    SurfaceGameplayStats, SurfaceMetricField, TraversalBlockReason,
};
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::BTreeMap;

pub struct SurfaceGameplayCompilerPlugin;

impl Plugin for SurfaceGameplayCompilerPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Compiles the authoritative gameplay layer.
///
/// Tile-set must match exactly: every metric cell has a tile and every tile
/// has metric cells (`MissingMetricsForTile` / `MetricWithoutTile`).
///
/// # Errors
/// Returns `SurfaceGameplayCompileError` on invalid config or tile-set
/// mismatch.
pub fn compile_surface_gameplay(
    field: &SurfaceMetricField,
    map_data: &MapData,
    config: &SurfaceGameplayConfig,
) -> Result<SurfaceGameplayMap, SurfaceGameplayCompileError> {
    config
        .validate_config()
        .map_err(SurfaceGameplayCompileError::InvalidConfig)?;

    for hex in field.cells.keys() {
        if !map_data.tiles.contains_key(hex) {
            return Err(SurfaceGameplayCompileError::MetricWithoutTile(*hex));
        }
    }
    for hex in map_data.tiles.keys() {
        if !field.cells.contains_key(hex) {
            return Err(SurfaceGameplayCompileError::MissingMetricsForTile(*hex));
        }
    }

    let mut cells = build_cells(field, map_data, config);
    apply_edge_build_gates(field, &mut cells, config);
    let edges = build_edges(field, &cells, config);
    let stats = compute_stats(&cells, &edges);

    Ok(SurfaceGameplayMap {
        cells,
        edges,
        stats,
    })
}

/// Cell policy: walkable (Land), terrain cost, buildable base gates.
fn build_cells(
    field: &SurfaceMetricField,
    map_data: &MapData,
    config: &SurfaceGameplayConfig,
) -> BTreeMap<HexCoord, SurfaceGameplayCell> {
    let mut cells = BTreeMap::new();
    for (hex, metric) in &field.cells {
        let Some(tile) = map_data.tiles.get(hex) else {
            continue;
        };
        let walkable = tile.ocean_state == OceanState::Land;
        let movement_cost = match tile.terrain {
            TerrainType::Swamp => config.swamp_cost,
            TerrainType::Stony => config.stony_cost,
            _ => config.walk_base_cost,
        };
        cells.insert(
            *hex,
            SurfaceGameplayCell {
                walkable,
                movement_cost,
                buildable: walkable
                    && tile.terrain.allows_buildings()
                    && metric.relief <= config.max_build_relief,
                center_xz: metric.center_xz,
                center_height: metric.center_height,
                relief: metric.relief,
            },
        );
    }
    cells
}

/// Buildability gates from incident edges: seams and neighbor height deltas.
fn apply_edge_build_gates(
    field: &SurfaceMetricField,
    cells: &mut BTreeMap<HexCoord, SurfaceGameplayCell>,
    config: &SurfaceGameplayConfig,
) {
    for (edge, edge_metric) in &field.edges {
        let Some(m_a) = field.cells.get(&edge.a) else {
            continue;
        };
        let Some(m_b) = field.cells.get(&edge.b) else {
            continue;
        };
        let neighbor_delta = (m_a.center_height - m_b.center_height).abs();
        if edge_metric.height_seam || neighbor_delta > config.max_build_neighbor_step {
            if let Some(cell) = cells.get_mut(&edge.a) {
                cell.buildable = false;
            }
            if let Some(cell) = cells.get_mut(&edge.b) {
                cell.buildable = false;
            }
        }
    }
}

/// Edge policy: seams always block; height steps block beyond `max_walk_step`.
fn build_edges(
    field: &SurfaceMetricField,
    cells: &BTreeMap<HexCoord, SurfaceGameplayCell>,
    config: &SurfaceGameplayConfig,
) -> BTreeMap<EdgeCoord, SurfaceGameplayEdge> {
    let mut edges = BTreeMap::new();
    for (edge, edge_metric) in &field.edges {
        let Some(cell_a) = cells.get(&edge.a) else {
            continue;
        };
        let Some(cell_b) = cells.get(&edge.b) else {
            continue;
        };
        let delta = (cell_a.center_height - cell_b.center_height).abs();
        let (traversable, block_reason) = if edge_metric.height_seam {
            (false, Some(TraversalBlockReason::CliffSeam))
        } else if delta > config.max_walk_step {
            (false, Some(TraversalBlockReason::HeightStep))
        } else {
            (true, None)
        };
        edges.insert(
            *edge,
            SurfaceGameplayEdge {
                traversable,
                block_reason,
                center_height_delta: delta,
            },
        );
    }
    edges
}

fn compute_stats(
    cells: &BTreeMap<HexCoord, SurfaceGameplayCell>,
    edges: &BTreeMap<EdgeCoord, SurfaceGameplayEdge>,
) -> SurfaceGameplayStats {
    SurfaceGameplayStats {
        cell_count: cells.len(),
        walkable_cell_count: cells.values().filter(|c| c.walkable).count(),
        buildable_cell_count: cells.values().filter(|c| c.buildable).count(),
        edge_count: edges.len(),
        traversable_edge_count: edges.values().filter(|e| e.traversable).count(),
        cliff_seam_edge_count: edges
            .values()
            .filter(|e| e.block_reason == Some(TraversalBlockReason::CliffSeam))
            .count(),
        height_step_edge_count: edges
            .values()
            .filter(|e| e.block_reason == Some(TraversalBlockReason::HeightStep))
            .count(),
    }
}
