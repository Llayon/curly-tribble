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
    Resources { resource: ResourceType, amount: u32 },
    Artifact(ArtifactType),
    // RAW ENTITY REMOVED: Use MapToTarget child with Targeting relation instead.
    TreasureMap,
}
pub type TargetEntity = Entity;

/// Relationship: This entity points to a Target Treasure.
/// Complies with Guard #18 (Semantic Graph).
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct Targeting {
    pub target: TargetEntity,
}

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
