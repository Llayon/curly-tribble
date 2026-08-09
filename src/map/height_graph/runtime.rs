// src/map/height_graph/runtime.rs
//! Bevy runtime plugin and system for Milestone M4.1 — Height Constraint Graph.

use crate::map::height_constraints::runtime::{
    HeightConstraintCompilationOutcome, HeightConstraintCompilationState,
};
use crate::map::height_constraints::types::HeightConstraintSet;
use crate::map::height_graph::builder::build_height_constraint_graph;
use crate::map::height_graph::types::HeightConstraintGraph;
use crate::map::height_graph::validation::validate_height_constraint_graph;
use crate::map::surface_topology::types::SurfaceTopology;
use crate::sets::GameSet;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeightGraphGenerationOutcome {
    #[default]
    Uninitialized,
    Success,
    Failure,
}

#[derive(Resource, Debug, Default)]
pub struct HeightGraphGenerationState {
    pub generation_count: u64,
    pub failure_count: u64,
    pub last_outcome: HeightGraphGenerationOutcome,
}

pub struct HeightGraphRuntimePlugin;

impl Plugin for HeightGraphRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeightGraphGenerationState>()
            .add_systems(
                Update,
                regenerate_height_constraint_graph
                    .in_set(GameSet::Visuals)
                    .after(crate::map::height_constraints::runtime::regenerate_height_constraints),
            );
    }
}

pub fn regenerate_height_constraint_graph(
    surface: Res<SurfaceTopology>,
    constraints: Res<HeightConstraintSet>,
    m4_state: Res<HeightConstraintCompilationState>,
    mut graph: ResMut<HeightConstraintGraph>,
    mut state: ResMut<HeightGraphGenerationState>,
) {
    if !surface.is_changed() && !constraints.is_changed() && !m4_state.is_changed() {
        return;
    }

    match m4_state.last_outcome {
        HeightConstraintCompilationOutcome::Uninitialized => return,
        HeightConstraintCompilationOutcome::Failure => {
            state.failure_count += 1;
            state.last_outcome = HeightGraphGenerationOutcome::Failure;
            *graph = HeightConstraintGraph::default();
            return;
        }
        HeightConstraintCompilationOutcome::Success => {}
    }

    match build_height_constraint_graph(&surface, &constraints) {
        Ok(new_graph) => {
            match validate_height_constraint_graph(&new_graph, &surface, &constraints) {
                Ok(()) => {
                    *graph = new_graph;
                    state.generation_count += 1;
                    state.last_outcome = HeightGraphGenerationOutcome::Success;
                }
                Err(_err) => {
                    state.failure_count += 1;
                    state.last_outcome = HeightGraphGenerationOutcome::Failure;
                    *graph = HeightConstraintGraph::default();
                }
            }
        }
        Err(_err) => {
            state.failure_count += 1;
            state.last_outcome = HeightGraphGenerationOutcome::Failure;
            *graph = HeightConstraintGraph::default();
        }
    }
}
