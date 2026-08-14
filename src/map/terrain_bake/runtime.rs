// src/map/terrain_bake/runtime.rs
//! Bevy runtime system and plugin for Milestone M5.1 — `SurfaceTerrainBake` lifecycle.

use crate::map::height_graph::types::HeightConstraintGraph;
use crate::map::surface_height::runtime::{
    HeightSolveGenerationOutcome, HeightSolveGenerationState,
};
use crate::map::surface_height::types::SurfaceHeightLayer;
use crate::map::surface_topology::types::SurfaceTopology;
use crate::map::terrain_bake::builder::build_surface_terrain_bake;
use crate::map::terrain_bake::types::SurfaceTerrainBake;
use crate::map::terrain_bake::validation::validate_surface_terrain_bake;
use crate::map::RebuildMeshEvent;
use crate::sets::GameSet;
use bevy::prelude::*;

pub struct TerrainBakeRuntimePlugin;

impl Plugin for TerrainBakeRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainBakeGenerationState>()
            .add_systems(
                Update,
                regenerate_surface_terrain_bake
                    .in_set(GameSet::Visuals)
                    .after(crate::map::surface_height::runtime::regenerate_surface_height_layer),
            );
    }
}

/// Rebuilds `SurfaceTerrainBake` whenever M5 produces a new `SurfaceHeightLayer`.
///
/// Uses a full-state `TerrainBakeSourceStamp` to distinguish all transitions:
/// - Uninitialized → no-op
/// - Same stamp → no-op (strict no-retry policy)
/// - Failure stamp → clear bake, no rebuild event
/// - Success stamp → build + validate + publish + `RebuildMeshEvent`
#[allow(clippy::too_many_arguments)]
pub fn regenerate_surface_terrain_bake(
    surface: Res<SurfaceTopology>,
    graph: Res<HeightConstraintGraph>,
    heights: Res<SurfaceHeightLayer>,
    height_state: Res<HeightSolveGenerationState>,
    mut bake: ResMut<SurfaceTerrainBake>,
    mut bake_state: ResMut<TerrainBakeGenerationState>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    // Gate 1: M5 uninitialized — pipeline not ready
    if height_state.last_outcome == HeightSolveGenerationOutcome::Uninitialized {
        return;
    }

    // Gate 2: construct full source stamp
    let stamp = TerrainBakeSourceStamp {
        generation_count: height_state.generation_count,
        failure_count: height_state.failure_count,
        outcome: height_state.last_outcome,
    };

    // Gate 3: same stamp as last processed — no retry (works for both Success and Failure)
    if bake_state.last_source == Some(stamp) {
        return;
    }

    // Record BEFORE work (strict no-retry: even if build panics, not retried)
    bake_state.last_source = Some(stamp);

    // Gate 4: M5 failure → clear bake once, no rebuild event
    if height_state.last_outcome == HeightSolveGenerationOutcome::Failure {
        *bake = SurfaceTerrainBake::default();
        bake_state.failure_count += 1;
        bake_state.last_outcome = TerrainBakeGenerationOutcome::Failure;
        return;
    }

    // M5 Success path: build + validate
    let result: Result<SurfaceTerrainBake, ()> = (|| {
        let new_bake = build_surface_terrain_bake(&surface, &graph, &heights).map_err(|_| ())?;
        validate_surface_terrain_bake(&new_bake, &surface, &graph, &heights).map_err(|_| ())?;
        Ok(new_bake)
    })();

    if let Ok(new_bake) = result {
        *bake = new_bake;
        bake_state.generation_count += 1;
        bake_state.last_outcome = TerrainBakeGenerationOutcome::Success;
        debug!(
            "SurfaceTerrainBake regenerated: vertices={} faces={} cliff_walls={} generation={}",
            bake.vertices.len(),
            bake.faces.len(),
            bake.cliff_walls.len(),
            bake_state.generation_count,
        );
        ev_rebuild.write(RebuildMeshEvent);
    } else {
        *bake = SurfaceTerrainBake::default();
        bake_state.failure_count += 1;
        bake_state.last_outcome = TerrainBakeGenerationOutcome::Failure;
        // NO rebuild event — old terrain remains
    }
}

// ─── State types ─────────────────────────────────────────────────────────────

/// Full snapshot of the M5 `HeightSolveGenerationState` at the time of processing.
/// Using both `generation_count` and `failure_count` ensures all transitions are
/// detected — including Success(gen=N) → Failure(gen=N) (`failure_count` differs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainBakeSourceStamp {
    pub generation_count: u64,
    pub failure_count: u64,
    pub outcome: HeightSolveGenerationOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerrainBakeGenerationOutcome {
    #[default]
    Uninitialized,
    Success,
    Failure,
}

#[derive(Resource, Debug, Default)]
pub struct TerrainBakeGenerationState {
    pub generation_count: u64,
    pub failure_count: u64,
    pub last_source: Option<TerrainBakeSourceStamp>,
    pub last_outcome: TerrainBakeGenerationOutcome,
}
