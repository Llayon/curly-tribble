// src/map/subhex.rs
use crate::economy::assets::GameAssets;
use crate::map::deposits::DepositType;
use crate::map::navigation::{NavObstacle, COST_BLOCKER};
use crate::map::{HexCoord, MapData, MapEntity, HEX_SIZE};
use bevy::prelude::*;

pub struct SubHexPlacementPlugin;

impl Plugin for SubHexPlacementPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct SubHexDeposit {
    pub deposit_type: DepositType,
    pub world_pos: Vec3,
}

#[derive(Bundle)]
pub struct SubHexDepositBundle {
    pub deposit: SubHexDeposit,
    pub scene: SceneRoot,
    pub transform: Transform,
    pub obstacle: NavObstacle,
    pub marker: MapEntity,
}

#[must_use]
pub fn snap_to_terrain_y(world_pos: Vec3, map_data: &MapData) -> Vec3 {
    let coord = HexCoord::from_world(world_pos, HEX_SIZE);
    let height = map_data.get_hex_height(coord.q, coord.r);
    Vec3::new(world_pos.x, height, world_pos.z)
}

pub fn spawn_subhex_deposit(
    commands: &mut Commands,
    world_pos: Vec3,
    deposit_type: DepositType,
    map_data: &MapData,
    assets: &GameAssets,
) -> Entity {
    let snapped_pos = snap_to_terrain_y(world_pos, map_data);

    let scene_handle = match deposit_type {
        DepositType::Raspberries | DepositType::Pumpkin | DepositType::WildWheat => {
            assets.bush_scene.clone()
        }
        _ => assets.tree_scene.clone(),
    };

    commands
        .spawn(SubHexDepositBundle {
            deposit: SubHexDeposit {
                deposit_type,
                world_pos: snapped_pos,
            },
            scene: SceneRoot(scene_handle),
            transform: Transform::from_translation(snapped_pos),
            obstacle: NavObstacle { cost: COST_BLOCKER },
            marker: MapEntity,
        })
        .id()
}
