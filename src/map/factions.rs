use crate::game_state::FactionType;
use crate::map::HexCoord;
use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug, Reflect)]
pub struct FactionMarker {
    pub faction_type: FactionType,
    pub hex_coord: HexCoord,
}

#[derive(Bundle)]
pub struct FactionMarkerBundle {
    pub marker: FactionMarker,
    pub name: Name,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
}
