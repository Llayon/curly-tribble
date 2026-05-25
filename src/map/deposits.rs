use crate::map::{HexCoord, MapEntity};
use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Default)]
pub enum DepositType {
    #[default]
    Rabbit,
    Deer,
    Boar,
    WildFlax,
    Raspberries,
    Pumpkin,
    WildWheat,
    OceanFish,
}

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct ResourceDeposit {
    pub deposit_type: DepositType,
    pub amount: u32,
    pub hex_coord: HexCoord,
    pub habitat_valid: bool,
}

#[derive(Bundle)]
pub struct ResourceDepositBundle {
    pub deposit: ResourceDeposit,
    pub name: Name,
    pub transform: Transform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub marker: MapEntity,
}
