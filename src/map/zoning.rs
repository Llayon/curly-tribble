use bevy::prelude::*;
use crate::map::data::{TerrainType, WorldSeed, MapData};
use crate::map::MapEntity;

#[derive(Component)]
pub struct Tile {
    pub terrain: TerrainType,
}

#[derive(Component)]
pub struct Roof;

#[derive(Bundle)]
pub struct RoofBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub roof: Roof,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct SmoothTileBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub tile: Tile,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct GlobalTerrainBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub name: Name,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct WaterBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub name: Name,
    pub marker: MapEntity,
}

#[derive(Bundle)]
pub struct MountainRoofBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub roof: Roof,
    pub name: Name,
    pub marker: MapEntity,
}

pub use super::poi::{PoiBundle, PoiType, PointOfInterest};
pub use super::camps::{EnemyCamp, EnemyCampBundle};
pub use super::deposits::{DepositType, ResourceDeposit, ResourceDepositBundle};
pub use super::factions::{FactionMarker, FactionMarkerBundle};

#[derive(Component)]
#[allow(dead_code)]
pub struct Zone(pub crate::map::data::ZoneType);

pub struct ZoningPlugin;

impl Plugin for ZoningPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldSeed>()
            .init_resource::<MapData>()
            .register_type::<PoiType>()
            .register_type::<PointOfInterest>()
            .register_type::<EnemyCamp>()
            .register_type::<ResourceDeposit>()
            .register_type::<DepositType>();
    }
}
