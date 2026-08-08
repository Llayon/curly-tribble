// src/map/height_constraints/runtime.rs
//! Authoritative runtime compilation and validation of `HeightConstraintSet`.

use crate::map::data::{CliffLowerSide, EdgeCoord, EdgeType, LandscapeFeature, MapData};
use crate::map::height_constraints::compiler::compile_height_constraints;
use crate::map::height_constraints::types::HeightConstraintSet;
use crate::map::height_constraints::validation::validate_height_constraint_set;
use crate::map::surface_topology::types::SurfaceTopology;
use crate::map::HexCoord;
use crate::sets::GameSet;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeightConstraintLogicalInputs {
    pub regions: Vec<(HexCoord, LandscapeFeature)>,
    pub cliffs: Vec<(EdgeCoord, CliffLowerSide)>,
}

impl HeightConstraintLogicalInputs {
    #[must_use]
    pub fn from_map(map_data: &MapData) -> Self {
        let mut regions: Vec<_> = map_data
            .tiles
            .iter()
            .filter(|(_, tile)| tile.landscape_feature != LandscapeFeature::None)
            .map(|(&hex, tile)| (hex, tile.landscape_feature))
            .collect();
        regions.sort_by_key(|(h, _)| (h.q, h.r));

        let mut cliffs: Vec<_> = map_data
            .edges
            .iter()
            .filter(|(_, edge_data)| edge_data.edge_type == EdgeType::Cliff)
            .map(|(&edge, edge_data)| (edge, edge_data.cliff_lower_side))
            .collect();
        cliffs.sort_by_key(|(e, _)| (e.a.q, e.a.r, e.b.q, e.b.r));

        Self { regions, cliffs }
    }
}

#[derive(Resource, Debug, Default)]
pub struct HeightConstraintCompilationState {
    pub generation_count: u64,
    pub failure_count: u64,
    pub last_inputs: Option<HeightConstraintLogicalInputs>,
}

#[allow(dead_code)]
pub struct HeightConstraintRuntimePlugin;

impl Plugin for HeightConstraintRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HeightConstraintCompilationState>()
            .add_systems(
                Update,
                regenerate_height_constraints
                    .in_set(GameSet::Visuals)
                    .after(crate::map::surface_topology::runtime::regenerate_surface_topology),
            );
    }
}

/// Regenerates derived `HeightConstraintSet` whenever persistent landscape intent or `SurfaceTopology` changes.
pub fn regenerate_height_constraints(
    map_data: Res<MapData>,
    surface: Res<SurfaceTopology>,
    mut constraints: ResMut<HeightConstraintSet>,
    mut state: ResMut<HeightConstraintCompilationState>,
) {
    if !map_data.is_changed() && !surface.is_changed() {
        return;
    }

    let inputs = HeightConstraintLogicalInputs::from_map(&map_data);

    if !surface.is_changed() && state.last_inputs.as_ref() == Some(&inputs) {
        return;
    }

    state.last_inputs = Some(inputs);

    match compile_height_constraints(&map_data, &surface) {
        Ok(new_constraints) => {
            match validate_height_constraint_set(&new_constraints, &map_data, &surface) {
                Ok(()) => {
                    bevy::log::tracing::event!(
                        bevy::log::tracing::Level::INFO,
                        regions = new_constraints.regions.len(),
                        cliffs = new_constraints.cliffs.len(),
                        referenced_faces = new_constraints.stats.referenced_surface_faces,
                        referenced_segments = new_constraints.stats.referenced_boundary_segments,
                        "HeightConstraintSet compiled and validated"
                    );
                    *constraints = new_constraints;
                    state.generation_count += 1;
                }
                Err(error) => {
                    state.failure_count += 1;
                    *constraints = HeightConstraintSet::default();
                    bevy::log::tracing::event!(
                        bevy::log::tracing::Level::ERROR,
                        error = ?error,
                        "HeightConstraintSet validation failed"
                    );
                }
            }
        }
        Err(error) => {
            state.failure_count += 1;
            *constraints = HeightConstraintSet::default();
            bevy::log::tracing::event!(
                bevy::log::tracing::Level::ERROR,
                error = ?error,
                "HeightConstraintSet compilation failed"
            );
        }
    }
}
