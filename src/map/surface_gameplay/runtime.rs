// src/map/surface_gameplay/runtime.rs
//! Milestone M6 — runtime system that keeps `SurfaceMetricField` and
//! `SurfaceGameplayMap` in sync with the M5.1 bake and the legacy map
//! classification, using a full input fingerprint and strict no-retry.

use crate::map::data::{MapData, OceanState};
use crate::map::surface_gameplay::compiler::compile_surface_gameplay;
use crate::map::surface_gameplay::config::{
    SurfaceGameplayConfig, SurfaceGameplayConfigFingerprint,
};
use crate::map::surface_gameplay::metrics::derive_surface_metrics;
use crate::map::surface_gameplay::types::{SurfaceGameplayMap, SurfaceMetricField};
use crate::map::surface_gameplay::validation::{
    validate_surface_gameplay_map, validate_surface_metric_field,
};
use crate::map::surface_topology::types::SurfaceTopology;
use crate::map::terrain_bake::runtime::{
    TerrainBakeGenerationOutcome, TerrainBakeGenerationState, TerrainBakeSourceStamp,
};
use crate::map::terrain_bake::types::SurfaceTerrainBake;
use crate::map::RebuildMeshEvent;
use crate::sets::GameSet;
use bevy::prelude::*;

pub struct SurfaceGameplayRuntimePlugin;

impl Plugin for SurfaceGameplayRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SurfaceGameplayGenerationState>()
            .add_systems(
                Update,
                regenerate_surface_gameplay
                    .in_set(GameSet::Visuals)
                    .after(crate::map::terrain_bake::runtime::regenerate_surface_terrain_bake),
            );
    }
}

/// Regenerates metrics + gameplay whenever the bake, terrain classification,
/// or config changes.
///
/// Strict no-retry: `last_attempt` is recorded BEFORE work. Bake failure
/// clears both resources without a rebuild event; bake success derives,
/// validates, compiles, validates again, then publishes both and emits
/// `RebuildMeshEvent`.
#[allow(clippy::too_many_arguments)]
pub fn regenerate_surface_gameplay(
    surface: Res<SurfaceTopology>,
    bake: Res<SurfaceTerrainBake>,
    bake_state: Res<TerrainBakeGenerationState>,
    map_data: Res<MapData>,
    config: Res<SurfaceGameplayConfig>,
    mut field: ResMut<SurfaceMetricField>,
    mut gameplay: ResMut<SurfaceGameplayMap>,
    mut state: ResMut<SurfaceGameplayGenerationState>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    // Gate 1: M5.1 uninitialized — pipeline not ready
    if bake_state.last_outcome == TerrainBakeGenerationOutcome::Uninitialized {
        return;
    }

    // Gate 2: skip when no source resource changed this frame
    if !surface.is_changed()
        && !bake.is_changed()
        && !bake_state.is_changed()
        && !map_data.is_changed()
        && !config.is_changed()
    {
        return;
    }

    // Gate 3: same fingerprint as last processed — no retry
    let fingerprint =
        SurfaceGameplayInputFingerprint::build(bake_state.last_source, &map_data, &config);
    if state.last_attempt.as_ref() == Some(&fingerprint) {
        return;
    }

    // Record BEFORE work (strict no-retry: even if derivation panics, not retried)
    state.last_attempt = Some(fingerprint);

    // Gate 4: bake failure → clear both resources, no rebuild event
    if bake_state.last_outcome == TerrainBakeGenerationOutcome::Failure {
        *field = SurfaceMetricField::default();
        *gameplay = SurfaceGameplayMap::default();
        state.failure_count += 1;
        state.last_outcome = SurfaceGameplayGenerationOutcome::Failure;
        return;
    }

    // Bake Success path: derive + validate + compile + validate + publish
    let result: Result<(SurfaceMetricField, SurfaceGameplayMap), ()> = (|| {
        let new_field = derive_surface_metrics(&surface, &bake).map_err(|_| ())?;
        validate_surface_metric_field(&new_field, &surface, &bake).map_err(|_| ())?;
        let new_map = compile_surface_gameplay(&new_field, &map_data, &config).map_err(|_| ())?;
        validate_surface_gameplay_map(&new_map, &new_field, &map_data, &config).map_err(|_| ())?;
        Ok((new_field, new_map))
    })();

    if let Ok((new_field, new_map)) = result {
        *field = new_field;
        *gameplay = new_map;
        state.generation_count += 1;
        state.last_outcome = SurfaceGameplayGenerationOutcome::Success;
        debug!(
            "SurfaceGameplay regenerated: cells={} edges={} generation={}",
            gameplay.cells.len(),
            gameplay.edges.len(),
            state.generation_count,
        );
        ev_rebuild.write(RebuildMeshEvent);
    } else {
        *field = SurfaceMetricField::default();
        *gameplay = SurfaceGameplayMap::default();
        state.failure_count += 1;
        state.last_outcome = SurfaceGameplayGenerationOutcome::Failure;
        // NO rebuild event — old terrain remains
    }
}

// ─── State types ─────────────────────────────────────────────────────────────

/// Full logical inputs of the gameplay derivation. `TerrainType` is NOT part
/// of the bake fingerprint, so terrain classification changes (e.g. sediment
/// tools) must trigger regeneration on their own — hence tiles are included.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceGameplayInputFingerprint {
    pub bake_stamp: Option<TerrainBakeSourceStamp>,
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<(crate::map::HexCoord, u8, u8)>,
    pub config: SurfaceGameplayConfigFingerprint,
}

impl SurfaceGameplayInputFingerprint {
    #[must_use]
    pub fn build(
        bake_stamp: Option<TerrainBakeSourceStamp>,
        map_data: &MapData,
        config: &SurfaceGameplayConfig,
    ) -> Self {
        let mut tiles: Vec<_> = map_data
            .tiles
            .iter()
            .map(|(&hex, tile)| {
                let terrain_bits = tile.terrain as u8;
                let ocean_val = match tile.ocean_state {
                    OceanState::Land => 0u8,
                    OceanState::Ocean => 1u8,
                };
                (hex, terrain_bits, ocean_val)
            })
            .collect();
        tiles.sort_by(|a, b| a.0.q.cmp(&b.0.q).then_with(|| a.0.r.cmp(&b.0.r)));

        Self {
            bake_stamp,
            width: map_data.width,
            height: map_data.height,
            tiles,
            config: SurfaceGameplayConfigFingerprint::from_config(config),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SurfaceGameplayGenerationOutcome {
    #[default]
    Uninitialized,
    Success,
    Failure,
}

#[derive(Resource, Debug, Default)]
pub struct SurfaceGameplayGenerationState {
    pub generation_count: u64,
    pub failure_count: u64,
    pub last_attempt: Option<SurfaceGameplayInputFingerprint>,
    pub last_outcome: SurfaceGameplayGenerationOutcome,
}
