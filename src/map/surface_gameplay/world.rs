// src/map/surface_gameplay/world.rs
//! Milestone M6 — world-space anchoring adapter. The ONLY place where
//! `MAX_HEIGHT` meets gameplay data: converts a gameplay cell to a world
//! position using the solved center height.

use crate::map::data::{HEX_SIZE, MAX_HEIGHT};
use crate::map::navigation::AGENT_HEIGHT;
use crate::map::surface_gameplay::types::SurfaceGameplayMap;
use crate::map::HexCoord;
use bevy::prelude::*;

pub struct SurfaceGameplayWorldPlugin;

impl Plugin for SurfaceGameplayWorldPlugin {
    fn build(&self, _app: &mut App) {}
}

/// World position of the solved center of `hex`, agent-height above the
/// solved surface. Missing cells fall back to height `0.0` (ocean-safe).
#[must_use]
pub fn gameplay_center_world_pos(hex: HexCoord, gameplay: &SurfaceGameplayMap) -> Vec3 {
    let mut pos = hex.to_world(HEX_SIZE);
    let height = gameplay
        .cells
        .get(&hex)
        .map_or(0.0, |cell| cell.center_height);
    pos.y = height * MAX_HEIGHT + AGENT_HEIGHT;
    pos
}
