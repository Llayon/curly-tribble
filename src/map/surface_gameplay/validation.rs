// src/map/surface_gameplay/validation.rs
//! Milestone M6 — independent recompute validators for the metrics field and
//! the gameplay map. Each validator re-derives its target from the source
//! resources and fails on any mismatch, catching non-determinism or external
//! mutation before publishing.

use crate::map::data::MapData;
use crate::map::surface_gameplay::compiler::compile_surface_gameplay;
use crate::map::surface_gameplay::config::SurfaceGameplayConfig;
use crate::map::surface_gameplay::metrics::derive_surface_metrics;
use crate::map::surface_gameplay::types::{
    SurfaceGameplayCompileError, SurfaceGameplayMap, SurfaceMetricField, SurfaceMetricsError,
};
use crate::map::surface_topology::types::SurfaceTopology;
use crate::map::terrain_bake::types::SurfaceTerrainBake;
use bevy::prelude::*;

pub struct SurfaceGameplayValidationPlugin;

impl Plugin for SurfaceGameplayValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceGameplayValidationError {
    MetricsRecompute(SurfaceMetricsError),
    Compile(SurfaceGameplayCompileError),
    FieldMismatch,
    MapMismatch,
}

/// Re-derives the metric field from the source topology + bake and compares
/// it with the published resource.
///
/// # Errors
/// Returns `SurfaceGameplayValidationError` on recompute failure or mismatch.
pub fn validate_surface_metric_field(
    field: &SurfaceMetricField,
    surface: &SurfaceTopology,
    bake: &SurfaceTerrainBake,
) -> Result<(), SurfaceGameplayValidationError> {
    let recomputed = derive_surface_metrics(surface, bake)
        .map_err(SurfaceGameplayValidationError::MetricsRecompute)?;
    if recomputed != *field {
        return Err(SurfaceGameplayValidationError::FieldMismatch);
    }
    Ok(())
}

/// Re-compiles the gameplay map from metrics + map data + config and compares
/// it with the published resource.
///
/// # Errors
/// Returns `SurfaceGameplayValidationError` on recompute failure or mismatch.
pub fn validate_surface_gameplay_map(
    map: &SurfaceGameplayMap,
    field: &SurfaceMetricField,
    map_data: &MapData,
    config: &SurfaceGameplayConfig,
) -> Result<(), SurfaceGameplayValidationError> {
    let recomputed = compile_surface_gameplay(field, map_data, config)
        .map_err(SurfaceGameplayValidationError::Compile)?;
    if recomputed != *map {
        return Err(SurfaceGameplayValidationError::MapMismatch);
    }
    Ok(())
}
