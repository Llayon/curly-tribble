use crate::economy::GameAssets;
use crate::map::{MapData, TerrainType, WorldSeed, HEX_SIZE};
use bevy::prelude::*;
use rand::prelude::*;

pub struct ResourcesPlugin;

impl Plugin for ResourcesPlugin {
    fn build(&self, app: &mut App) {
        // Запускаем после того, как карта создана
        app.add_systems(PostStartup, spawn_resources);
    }
}

#[derive(Component)]
pub struct BerryBush {
    pub food_amount: f32,
}

#[derive(Bundle)]
pub struct BerryBushBundle {
    pub bush: BerryBush,
    pub scene: SceneRoot,
    pub transform: Transform,
    pub obstacle: crate::map::navigation::NavObstacle,
}

#[derive(Component)]
pub struct Tree;

#[derive(Bundle)]
pub struct TreeBundle {
    pub tree: Tree,
    pub scene: SceneRoot,
    pub transform: Transform,
    pub obstacle: crate::map::navigation::NavObstacle,
}

#[derive(Component)]
pub struct Rock;

#[derive(Bundle)]
pub struct RockBundle {
    pub rock: Rock,
    pub scene: SceneRoot,
    pub transform: Transform,
    pub obstacle: crate::map::navigation::NavObstacle,
}

fn spawn_resources(
    mut commands: Commands,
    assets: Res<GameAssets>,
    map_data: Res<MapData>,
    seed: Res<WorldSeed>,
) {
    let mut rng = StdRng::seed_from_u64(u64::from(seed.value()) + 42);
    let Ok(width) = i32::try_from(map_data.width) else {
        return;
    };
    let Ok(height) = i32::try_from(map_data.height) else {
        return;
    };
    let half_w = width / 2;
    let half_h = height / 2;

    for q in -half_w..half_w {
        for r in -half_h..half_h {
            let Some(tile) = map_data.get_tile(q, r) else {
                continue;
            };

            let mut pos = crate::map::HexCoord::new(q, r).to_world(HEX_SIZE);
            pos.y = map_data.get_hex_height(q, r);

            match tile.terrain {
                TerrainType::Grass => {
                    let tree_chance = if tile.humidity > 0.6 { 0.25 } else { 0.05 };
                    if rng.gen_bool(tree_chance) {
                        commands.spawn(TreeBundle {
                            tree: Tree,
                            scene: SceneRoot(assets.tree_scene.clone()),
                            transform: Transform::from_translation(pos),
                            obstacle: crate::map::navigation::NavObstacle {
                                cost: crate::map::navigation::COST_BLOCKER,
                            },
                        });
                    } else if rng.gen_bool(0.08) {
                        commands.spawn(BerryBushBundle {
                            bush: BerryBush { food_amount: 10.0 },
                            scene: SceneRoot(assets.bush_scene.clone()),
                            transform: Transform::from_translation(pos),
                            obstacle: crate::map::navigation::NavObstacle {
                                cost: crate::map::navigation::COST_BLOCKER,
                            },
                        });
                    }
                }
                TerrainType::Swamp if rng.gen_bool(0.15) => {
                    commands.spawn(BerryBushBundle {
                        bush: BerryBush { food_amount: 10.0 },
                        scene: SceneRoot(assets.bush_scene.clone()),
                        transform: Transform::from_translation(pos),
                        obstacle: crate::map::navigation::NavObstacle {
                            cost: crate::map::navigation::COST_BLOCKER,
                        },
                    });
                }
                TerrainType::Stony if rng.gen_bool(0.12) => {
                    commands.spawn(RockBundle {
                        rock: Rock,
                        scene: SceneRoot(assets.rock_scene.clone()),
                        transform: Transform::from_translation(pos),
                        obstacle: crate::map::navigation::NavObstacle {
                            cost: crate::map::navigation::COST_BLOCKER,
                        },
                    });
                }
                _ => {}
            }
        }
    }
}
