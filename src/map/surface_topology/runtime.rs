// src/map/surface_topology/runtime.rs
//! Authoritative runtime regeneration of the semantic `SurfaceTopology` resource.

use super::provenance_validation::validate_surface_provenance;
use super::validation::{validate_fixed24_surface_topology, validate_surface_topology};
use crate::map::face_topology::types::HexFaceTopology;
use crate::map::surface_topology::generator::generate_surface_topology;
use crate::map::surface_topology::types::SurfaceTopology;
use crate::sets::GameSet;
use bevy::prelude::*;

#[derive(Resource, Debug, Default)]
pub struct SurfaceTopologyGenerationState {
    pub generation_count: u64,
    pub failure_count: u64,
}

pub struct SurfaceTopologyRuntimePlugin;

impl Plugin for SurfaceTopologyRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SurfaceTopologyGenerationState>()
            .add_systems(
                Update,
                regenerate_surface_topology
                    .after(crate::map::face_topology::runtime::regenerate_hex_face_topology)
                    .in_set(GameSet::Visuals),
            );
    }
}

fn run_surface_validation(
    surface: &SurfaceTopology,
    face_topology: &HexFaceTopology,
) -> Result<(), crate::map::surface_topology::types::SurfaceTopologyError> {
    validate_surface_topology(surface)?;
    validate_fixed24_surface_topology(surface, face_topology)?;
    validate_surface_provenance(surface, face_topology)?;
    Ok(())
}

/// Regenerates the derived semantic `SurfaceTopology` whenever authoritative `HexFaceTopology` changes.
pub fn regenerate_surface_topology(
    face_topology: Res<HexFaceTopology>,
    mut surface_topology: ResMut<SurfaceTopology>,
    mut generation_state: ResMut<SurfaceTopologyGenerationState>,
) {
    if !face_topology.is_changed() {
        return;
    }

    if face_topology.faces.is_empty() {
        *surface_topology = SurfaceTopology::default();
        return;
    }

    match generate_surface_topology(&face_topology) {
        Ok(new_surface) => match run_surface_validation(&new_surface, &face_topology) {
            Ok(()) => {
                bevy::log::tracing::event!(
                    bevy::log::tracing::Level::INFO,
                    vertices = new_surface.vertices.len(),
                    faces = new_surface.faces.len(),
                    half_edges = new_surface.half_edges.len(),
                    paired_half_edges = new_surface.stats.paired_half_edge_count,
                    boundary_half_edges = new_surface.stats.boundary_half_edge_count,
                    "SurfaceTopology regenerated and validated"
                );
                *surface_topology = new_surface;
                generation_state.generation_count += 1;
            }
            Err(error) => {
                generation_state.failure_count += 1;
                *surface_topology = SurfaceTopology::default();
                bevy::log::tracing::event!(
                    bevy::log::tracing::Level::ERROR,
                    error = ?error,
                    "SurfaceTopology validation failed"
                );
            }
        },
        Err(error) => {
            generation_state.failure_count += 1;
            *surface_topology = SurfaceTopology::default();
            bevy::log::tracing::event!(
                bevy::log::tracing::Level::ERROR,
                error = ?error,
                "SurfaceTopology generation failed"
            );
        }
    }
}
