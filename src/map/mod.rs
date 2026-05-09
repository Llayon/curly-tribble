use crate::sets::StartupSet;
use bevy::prelude::*;
use construction::ConstructionPlugin;
use navigation::{NavigationPlugin, NavObstacle};
use resources::ResourcesPlugin;
use zoning::{TerrainType, Tile};
use noise::{NoiseFn, Fbm, Perlin};

pub mod atmosphere;
pub mod construction;
pub mod navigation;
pub mod resources;
pub mod zoning;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldSeed>()
            .add_plugins((
                zoning::ZoningPlugin,
                ResourcesPlugin,
                ConstructionPlugin,
                NavigationPlugin,
            ))
            .add_systems(Startup, spawn_map.in_set(StartupSet::SpawnEntities));
    }
}

#[derive(Resource)]
pub struct WorldSeed(pub u32);

impl Default for WorldSeed {
    fn default() -> Self {
        Self(42) // Тот самый сид
    }
}

#[derive(Bundle)]
pub struct MapTileBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub tile: Tile,
}

fn spawn_map(
    mut commands: Commands, 
    assets: Res<crate::economy::GameAssets>,
    seed: Res<WorldSeed>,
) {
    let fbm = Fbm::<Perlin>::new(seed.0);
    
    // Увеличим карту до 20x20 для интереса
    for x in -10..10 {
        for z in -10..10 {
            // Генерируем значение шума (-1.0 .. 1.0)
            let val = fbm.get([x as f64 * 0.2, z as f64 * 0.2]);
            
            let terrain = if val < -0.3 {
                TerrainType::Water
            } else if val < 0.2 {
                TerrainType::Grass
            } else {
                TerrainType::Mud
            };

            let material = match terrain {
                TerrainType::Grass => assets.ground_material.clone(),
                TerrainType::Mud => assets.mud_material.clone(),
                TerrainType::Water => assets.water_material.clone(),
            };

            let mut entity = commands.spawn(MapTileBundle {
                mesh: Mesh3d(assets.ground_mesh.clone()),
                material: MeshMaterial3d(material),
                transform: Transform::from_xyz(x as f32, 0.0, z as f32),
                tile: Tile { terrain },
            });

            // Добавляем препятствия/стоимость в навигацию
            match terrain {
                TerrainType::Water => {
                    entity.insert(NavObstacle { cost: 0 }); // Блокер
                }
                TerrainType::Mud => {
                    entity.insert(NavObstacle { cost: 50 }); // Замедление
                }
                TerrainType::Grass => {} // Базовая стоимость 20
            }
        }
    }
}
