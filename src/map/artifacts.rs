use crate::map::treasures::{ArtifactType, ResourceType};
use crate::map::HexCoord;
use bevy::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub struct TradeConfig {
    pub faction_id: u32,
    pub cost_type: ResourceType,
    pub cost_amount: u32,
    pub unlock_condition: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub enum ArtifactLocation {
    InTreasure,
    OnGround(HexCoord),
    InTrade(TradeConfig),
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
#[relationship(relationship_target = StoresArtifacts)]
pub struct StoredInTreasure(pub Entity);

#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
#[relationship_target(relationship = StoredInTreasure)]
pub struct StoresArtifacts(Vec<Entity>);

#[derive(Component, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct Artifact {
    pub artifact_type: ArtifactType,
    pub location: ArtifactLocation,
}

#[derive(Bundle)]
pub struct ArtifactBundle {
    pub artifact: Artifact,
    pub name: Name,
    pub marker: crate::map::MapEntity,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
}
