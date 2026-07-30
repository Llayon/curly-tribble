// src/map/roads.rs
use crate::economy::assets::GameAssets;
use crate::map::data::OceanState;
use crate::map::{HexCoord, MapData, MapEntity, TerrainType, HEX_SIZE};
use bevy::prelude::*;

pub struct RoadsPlugin;

impl Plugin for RoadsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Reflect, Default)]
#[reflect(Component)]
pub struct RoadTile;

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct BorderPost {
    pub faction_id: u32,
    pub hex_coord: HexCoord,
}

#[derive(Bundle)]
pub struct BorderPostBundle {
    pub post: BorderPost,
    pub name: Name,
    pub scene: SceneRoot,
    pub transform: Transform,
    pub marker: MapEntity,
}

#[allow(clippy::cast_possible_truncation)]
pub fn generate_village_roads(map_data: &mut MapData, points: &[HexCoord]) {
    if points.len() < 2 {
        return;
    }

    for window in points.windows(2) {
        let start = window[0];
        let end = window[1];

        // Draw line of hexes between start and end
        let n = start.distance(end);
        if n == 0 {
            continue;
        }

        for i in 0..=n {
            let t = i as f32 / n as f32;
            let lerp_q = (start.q as f32 * (1.0 - t) + end.q as f32 * t).round() as i32;
            let lerp_r = (start.r as f32 * (1.0 - t) + end.r as f32 * t).round() as i32;
            let coord = HexCoord::new(lerp_q, lerp_r);

            if let Some(tile) = map_data.tiles.get_mut(&coord) {
                if tile.ocean_state == OceanState::Land {
                    tile.terrain = TerrainType::Dirt;
                }
            }
        }
    }
}

pub fn spawn_faction_border_posts(
    commands: &mut Commands,
    map_data: &MapData,
    assets: &GameAssets,
) -> Vec<Entity> {
    let mut spawned = Vec::new();

    for (coord, tile) in &map_data.tiles {
        let Some(faction_id) = tile.faction_id else {
            continue;
        };

        // Check if any neighbor belongs to a different faction or neutral land
        let is_border = coord.neighbors().iter().any(|n| {
            if let Some(neighbor_tile) = map_data.tiles.get(n) {
                neighbor_tile.faction_id != Some(faction_id)
                    && neighbor_tile.ocean_state == OceanState::Land
            } else {
                false
            }
        });

        if is_border {
            let height = map_data.get_hex_height(coord.q, coord.r);
            let mut pos = coord.to_world(HEX_SIZE);
            pos.y = height;

            let entity = commands
                .spawn(BorderPostBundle {
                    post: BorderPost {
                        faction_id,
                        hex_coord: *coord,
                    },
                    name: Name::new(format!("Faction {faction_id} Border Post")),
                    scene: SceneRoot(assets.tree_scene.clone()),
                    transform: Transform::from_translation(pos),
                    marker: MapEntity,
                })
                .id();

            spawned.push(entity);
        }
    }

    spawned
}
