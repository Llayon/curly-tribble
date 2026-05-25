use crate::map::{HexCoord, MapEntity};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum PoiType {
    #[default]
    TradePost,
    Ruins,
    Shrine,
    Treasure,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct PointOfInterest {
    pub poi_type: PoiType,
    pub hex_coord: HexCoord,
    pub linked_objective_id: Option<u32>,
}

#[derive(Bundle)]
pub struct PoiBundle {
    pub poi: PointOfInterest,
    pub name: Name,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub marker: MapEntity,
}
