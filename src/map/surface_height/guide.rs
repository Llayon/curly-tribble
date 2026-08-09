// src/map/surface_height/guide.rs
//! Compatibility adapter deriving `LegacyHeightGuide` from `MapData`, `SurfaceTopology`, and `HeightConstraintGraph`.

use crate::map::data::{MapData, OceanState};
use crate::map::height_graph::types::{HeightConstraintGraph, HeightNodeId};
use crate::map::surface_topology::types::{SurfaceFaceId, SurfaceTopology};
use crate::map::HexCoord;
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeightGuideSample {
    pub target: f32,
    pub hard_pin: Option<f32>,
}

#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct LegacyHeightGuide {
    pub samples: Vec<HeightGuideSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeightGuideError {
    NodeCountMismatch,
    InvalidSurfaceFace(SurfaceFaceId),
    MissingOwnerTile(HexCoord),
    HeightNodeWithoutOwners(HeightNodeId),
    NonFiniteElevation { hex: HexCoord },
    ElevationOutOfRange { hex: HexCoord },
}

/// Derives deterministic legacy height guide for all nodes in `HeightConstraintGraph`.
///
/// # Errors
/// Returns `HeightGuideError` if topology references missing faces/tiles or malformed elevation.
#[allow(clippy::too_many_lines)]
pub fn derive_legacy_height_guide(
    map_data: &MapData,
    surface: &SurfaceTopology,
    graph: &HeightConstraintGraph,
) -> Result<LegacyHeightGuide, HeightGuideError> {
    let mut samples = Vec::with_capacity(graph.nodes.len());

    for (node_idx, node) in graph.nodes.iter().enumerate() {
        let node_id = HeightNodeId::new(node_idx);

        if node.incident_faces.is_empty() {
            return Err(HeightGuideError::HeightNodeWithoutOwners(node_id));
        }

        // Canonical owner resolution: incident faces -> owner hexes -> sort (q,r) -> dedup
        let mut owner_hexes: Vec<HexCoord> = Vec::with_capacity(node.incident_faces.len());
        for &face_id in &node.incident_faces {
            let face = surface
                .faces
                .get(face_id.index())
                .ok_or(HeightGuideError::InvalidSurfaceFace(face_id))?;
            owner_hexes.push(face.owner_hex);
        }
        owner_hexes.sort_by(|a, b| a.q.cmp(&b.q).then_with(|| a.r.cmp(&b.r)));
        owner_hexes.dedup();

        let mut sum_elevation = 0.0f32;
        let mut ocean_count = 0usize;
        let total_owners = owner_hexes.len();

        for &hex in &owner_hexes {
            let tile = map_data
                .tiles
                .get(&hex)
                .ok_or(HeightGuideError::MissingOwnerTile(hex))?;

            if !tile.elevation.is_finite() {
                return Err(HeightGuideError::NonFiniteElevation { hex });
            }
            if !(0.0..=1.0).contains(&tile.elevation) {
                return Err(HeightGuideError::ElevationOutOfRange { hex });
            }

            if tile.ocean_state == OceanState::Ocean {
                ocean_count += 1;
                // Ocean owner contributes 0.0 to sample sum
            } else {
                sum_elevation += tile.elevation;
            }
        }

        let target = sum_elevation / (total_owners as f32);
        let hard_pin = if ocean_count == total_owners {
            Some(0.0)
        } else {
            None
        };

        samples.push(HeightGuideSample { target, hard_pin });
    }

    Ok(LegacyHeightGuide { samples })
}

#[allow(dead_code)]
pub struct HeightGuidePlugin;

impl Plugin for HeightGuidePlugin {
    fn build(&self, _app: &mut App) {}
}
