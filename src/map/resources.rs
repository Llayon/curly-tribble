use crate::economy::GameAssets;
use crate::sets::StartupSet;
use crate::map::zoning::{TerrainType, Tile};
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

fn spawn_resources(
    mut commands: Commands, 
    assets: Res<GameAssets>,
    tiles: Query<(&Transform, &Tile)>,
) {
    let mut rng = StdRng::seed_from_u64(42);

    for (transform, tile) in &tiles {
        if tile.terrain != TerrainType::Grass {
            continue;
        }

        let spawn_chance = rng.gen_bool(0.1); // 10% шанс на объект

        if spawn_chance {
            let pos = transform.translation + Vec3::Y * 0.4;
            
            if rng.gen_bool(0.3) {
                // Ягодный куст
                commands.spawn(BerryBushBundle {
                    bush: BerryBush { food_amount: 10.0 },
                    scene: SceneRoot(assets.bush_scene.clone()),
                    transform: Transform::from_translation(pos),
                    obstacle: crate::map::navigation::NavObstacle {
                        cost: crate::map::navigation::COST_BLOCKER,
                    },
                });
            } else {
                // Просто декоративное дерево
                commands.spawn((
                    Tree,
                    SceneRoot(assets.tree_scene.clone()),
                    Transform::from_translation(pos),
                    crate::map::navigation::NavObstacle {
                        cost: crate::map::navigation::COST_BLOCKER,
                    },
                ));
            }
        }
    }
}
