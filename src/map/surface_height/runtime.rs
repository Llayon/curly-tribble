// src/map/surface_height/runtime.rs
//! Bevy runtime system and plugin for Milestone M5 — `SurfaceHeightLayer`.

use crate::map::data::{MapData, OceanState};
use crate::map::height_graph::runtime::{HeightGraphGenerationOutcome, HeightGraphGenerationState};
use crate::map::height_graph::types::HeightConstraintGraph;
use crate::map::surface_height::guide::{derive_legacy_height_guide, LegacyHeightGuide};
use crate::map::surface_height::hard_constraints::compile_hard_constraints;
use crate::map::surface_height::solver::solve_surface_heights;
use crate::map::surface_height::targets::compile_height_targets;
use crate::map::surface_height::types::{HeightSolverConfig, SurfaceHeightLayer};
use crate::map::surface_height::validation::validate_surface_height_layer;
use crate::map::surface_topology::types::SurfaceTopology;
use crate::map::HexCoord;
use crate::sets::GameSet;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HeightSolverConfigFingerprint {
    pub guide_weight_bits: u32,
    pub region_weight_bits: u32,
    pub smoothness_weight_bits: u32,
    pub mountain_bias_bits: u32,
    pub plateau_bias_bits: u32,
    pub lake_bias_bits: u32,
    pub river_bias_bits: u32,
    pub cliff_min_drop_bits: u32,
    pub relaxation_bits: u32,
    pub max_iterations: u32,
    pub convergence_epsilon_bits: u32,
}

impl HeightSolverConfigFingerprint {
    #[must_use]
    pub fn from_config(c: &HeightSolverConfig) -> Self {
        Self {
            guide_weight_bits: c.guide_weight.to_bits(),
            region_weight_bits: c.region_weight.to_bits(),
            smoothness_weight_bits: c.smoothness_weight.to_bits(),
            mountain_bias_bits: c.mountain_bias.to_bits(),
            plateau_bias_bits: c.plateau_bias.to_bits(),
            lake_bias_bits: c.lake_bias.to_bits(),
            river_bias_bits: c.river_bias.to_bits(),
            cliff_min_drop_bits: c.cliff_min_drop.to_bits(),
            relaxation_bits: c.relaxation.to_bits(),
            max_iterations: c.max_iterations,
            convergence_epsilon_bits: c.convergence_epsilon.to_bits(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeightSolveLogicalInputs {
    pub graph_generation: u64,
    pub tiles: Vec<(HexCoord, u32, u8)>,
    pub config: HeightSolverConfigFingerprint,
}

impl HeightSolveLogicalInputs {
    #[must_use]
    pub fn build(graph_gen: u64, map_data: &MapData, config: &HeightSolverConfig) -> Self {
        let mut tiles: Vec<_> = map_data
            .tiles
            .iter()
            .map(|(&hex, tile)| {
                let ocean_val = match tile.ocean_state {
                    OceanState::Land => 0u8,
                    OceanState::Ocean => 1u8,
                };
                (hex, tile.elevation.to_bits(), ocean_val)
            })
            .collect();
        tiles.sort_by(|a, b| a.0.q.cmp(&b.0.q).then_with(|| a.0.r.cmp(&b.0.r)));

        Self {
            graph_generation: graph_gen,
            tiles,
            config: HeightSolverConfigFingerprint::from_config(config),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeightSolveGenerationOutcome {
    #[default]
    Uninitialized,
    Success,
    Failure,
}

#[derive(Resource, Debug, Default)]
pub struct HeightSolveGenerationState {
    pub generation_count: u64,
    pub failure_count: u64,
    pub last_attempt: Option<HeightSolveLogicalInputs>,
    pub last_outcome: HeightSolveGenerationOutcome,
}

pub struct SurfaceHeightRuntimePlugin;

impl Plugin for SurfaceHeightRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeightSolveGenerationState>()
            .add_systems(
                Update,
                regenerate_surface_height_layer
                    .in_set(GameSet::Visuals)
                    .after(crate::map::height_graph::runtime::regenerate_height_constraint_graph),
            );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn regenerate_surface_height_layer(
    map_data: Res<MapData>,
    surface: Res<SurfaceTopology>,
    graph: Res<HeightConstraintGraph>,
    graph_state: Res<HeightGraphGenerationState>,
    config: Res<HeightSolverConfig>,
    mut guide: ResMut<LegacyHeightGuide>,
    mut layer: ResMut<SurfaceHeightLayer>,
    mut state: ResMut<HeightSolveGenerationState>,
) {
    if !surface.is_changed()
        && !graph.is_changed()
        && !graph_state.is_changed()
        && !config.is_changed()
        && !map_data.is_changed()
    {
        return;
    }

    match graph_state.last_outcome {
        HeightGraphGenerationOutcome::Uninitialized => return,
        HeightGraphGenerationOutcome::Failure => {
            state.failure_count += 1;
            state.last_outcome = HeightSolveGenerationOutcome::Failure;
            *guide = LegacyHeightGuide::default();
            *layer = SurfaceHeightLayer::default();
            return;
        }
        HeightGraphGenerationOutcome::Success => {}
    }

    let current_inputs =
        HeightSolveLogicalInputs::build(graph_state.generation_count, &map_data, &config);
    if state.last_attempt.as_ref() == Some(&current_inputs) {
        return;
    }

    // Strict no-retry policy: record attempt BEFORE work
    state.last_attempt = Some(current_inputs);

    let result: Result<(LegacyHeightGuide, SurfaceHeightLayer), ()> = (|| {
        config.validate_config().map_err(|_| ())?;
        let derived_guide =
            derive_legacy_height_guide(&map_data, &surface, &graph).map_err(|_| ())?;
        let compiled_targets =
            compile_height_targets(&graph, &derived_guide, &config).map_err(|_| ())?;
        let compiled_constraints =
            compile_hard_constraints(&graph, &derived_guide, &config).map_err(|_| ())?;
        let solved_layer = solve_surface_heights(
            &graph,
            &derived_guide,
            &compiled_targets,
            &compiled_constraints,
            &config,
        )
        .map_err(|_| ())?;
        validate_surface_height_layer(
            &solved_layer,
            &graph,
            &derived_guide,
            &compiled_constraints,
            &config,
        )
        .map_err(|_| ())?;
        Ok((derived_guide, solved_layer))
    })();

    if let Ok((new_guide, new_layer)) = result {
        *guide = new_guide;
        *layer = new_layer;
        state.generation_count += 1;
        state.last_outcome = HeightSolveGenerationOutcome::Success;
    } else {
        state.failure_count += 1;
        state.last_outcome = HeightSolveGenerationOutcome::Failure;
        *guide = LegacyHeightGuide::default();
        *layer = SurfaceHeightLayer::default();
    }
}
