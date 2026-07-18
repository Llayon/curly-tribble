use crate::map::{HexCoord, MapEntity};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ResourceType {
    Wood,
    Stone,
    Iron,
    Gold,
    Food,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ArtifactType {
    AncientRelic,
    TradeLedger,
    MagicCompass,
}

#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub enum TreasureItem {
    Gold(u32),
    Resources(ResourceType, u32),
    ArtifactDef(ArtifactType),
    ArtifactRef(TargetEntity),
    // RAW ENTITY REMOVED: Use MapToTarget child with Targeting relation instead.
    TreasureMap,
}
pub type TargetEntity = Entity;

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
#[relationship(relationship_target = ContainedByTreasure)]
pub struct ContainsArtifact(pub Entity);

#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
#[relationship_target(relationship = ContainsArtifact)]
pub struct ContainedByTreasure(Vec<Entity>);

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
#[relationship(relationship_target = TargetedByTreasureMap)]
pub struct Targeting(pub Entity);

/// Reverse index maintained by Bevy for every treasure map that targets this treasure.
#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
#[relationship_target(relationship = Targeting)]
pub struct TargetedByTreasureMap(Vec<Entity>);

/// Component for a child entity that represents a physical map item.
#[derive(Component, Debug, Clone, Reflect, Default)]
#[reflect(Component)]
pub struct MapToTarget;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Resource)]
pub enum LinkToolState {
    #[default]
    Idle,
    /// Reference to the source treasure.
    SelectingTarget(TargetEntity),
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
#[reflect(Component)]
pub struct VisibleTreasure;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
#[reflect(Component)]
pub struct HiddenTreasure;

#[derive(Component, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Component)]
pub struct TreasureDeposit {
    pub contents: Vec<TreasureItem>,
    pub hex_coord: HexCoord,
}

#[derive(Bundle)]
pub struct TreasureBundle {
    pub deposit: TreasureDeposit,
    pub name: Name,
    pub map_entity: MapEntity,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
}
