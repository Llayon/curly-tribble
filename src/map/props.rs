// src/map/props.rs
use crate::economy::assets::GameAssets;
use crate::map::{HexCoord, MapData, MapEntity, HEX_SIZE};
use bevy::prelude::*;

pub struct PropsPlugin;

impl Plugin for PropsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum PropType {
    #[default]
    Seashell,
    FlowerField,
    Pebbles,
    DecorativeBush,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
pub enum PropSnapMode {
    #[default]
    Land,
    Water,
}

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct DecorativeProp {
    pub prop_type: PropType,
    pub snap_mode: PropSnapMode,
    pub world_pos: Vec3,
}

#[derive(Bundle)]
pub struct DecorativePropBundle {
    pub prop: DecorativeProp,
    pub name: Name,
    pub scene: SceneRoot,
    pub transform: Transform,
    pub marker: MapEntity,
}

#[must_use]
pub fn snap_prop_y(world_pos: Vec3, snap_mode: PropSnapMode, map_data: &MapData) -> Vec3 {
    let y = match snap_mode {
        PropSnapMode::Water => 0.0,
        PropSnapMode::Land => {
            let coord = HexCoord::from_world(world_pos, HEX_SIZE);
            map_data.get_hex_height(coord.q, coord.r)
        }
    };
    Vec3::new(world_pos.x, y, world_pos.z)
}

pub fn spawn_decorative_prop(
    commands: &mut Commands,
    world_pos: Vec3,
    prop_type: PropType,
    snap_mode: PropSnapMode,
    map_data: &MapData,
    assets: &GameAssets,
) -> Entity {
    let snapped_pos = snap_prop_y(world_pos, snap_mode, map_data);

    let scene_handle = match prop_type {
        PropType::DecorativeBush => assets.bush_scene.clone(),
        _ => assets.tree_scene.clone(),
    };

    commands
        .spawn(DecorativePropBundle {
            prop: DecorativeProp {
                prop_type,
                snap_mode,
                world_pos: snapped_pos,
            },
            name: Name::new(format!("{prop_type:?} Prop")),
            scene: SceneRoot(scene_handle),
            transform: Transform::from_translation(snapped_pos),
            marker: MapEntity,
        })
        .id()
}
