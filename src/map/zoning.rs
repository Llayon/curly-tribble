use bevy::prelude::*;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneType {
    Empty,
    FoodStockpile,
    Housing,
}

#[derive(Component)]
pub struct Tile;

#[derive(Component)]
pub struct Zone(pub ZoneType);

pub struct ZoningPlugin;

impl Plugin for ZoningPlugin {
    fn build(&self, _app: &mut App) {
        // Here we could register types for reflection if needed
        // app.register_type::<Zone>();
    }
}
