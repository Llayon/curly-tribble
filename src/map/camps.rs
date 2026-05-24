use bevy::prelude::*;
use crate::map::{HexCoord, MapEntity};

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct EnemyCamp {
    pub hex_coord: HexCoord,
    pub sub_faction: String,
    pub difficulty: f32,
    pub combat_power: u32,
    pub camp_count: u32,
}

#[derive(Bundle)]
pub struct EnemyCampBundle {
    pub camp: EnemyCamp,
    pub name: Name,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub marker: MapEntity,
}
