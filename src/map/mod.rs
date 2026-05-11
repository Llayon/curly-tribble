use crate::economy::mesh_gen::MeshGenPlugin;
use crate::sets::StartupSet;
use bevy::prelude::*;
use construction::ConstructionPlugin;
use navigation::NavigationPlugin;
use noise::{Fbm, NoiseFn, Perlin};
use rand::prelude::*;
use resources::ResourcesPlugin;
use terrain_gen::TerrainGenerator;
pub use zoning::{MapData, TerrainType, Tile, WorldSeed, MAX_HEIGHT};

pub mod atmosphere;
pub mod construction;
pub mod navigation;
pub mod resources;
pub mod terrain_gen;
pub mod visibility;
pub mod zoning;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let seed_val = 42; // or get from WorldSeed if already initialized
        app.insert_resource(TerrainGenerator::new(seed_val))
            .add_plugins((
                zoning::ZoningPlugin,
                ResourcesPlugin,
                ConstructionPlugin,
                NavigationPlugin,
                visibility::VisibilityPlugin,
                MeshGenPlugin,
            ))
            .add_systems(Startup, spawn_map.in_set(StartupSet::SpawnEntities));
    }
}

#[derive(Bundle)]
pub struct MapTileBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub tile: Tile,
}

#[allow(clippy::cast_possible_truncation)] // Noise output f64 to f32 is intentional for terrain climate
fn spawn_map(
    mut commands: Commands,
    _assets: Res<crate::economy::GameAssets>,
    terrain_gen: Res<TerrainGenerator>,
    seed: Res<WorldSeed>,
    mut map_data: ResMut<MapData>,
    mut nav_map: ResMut<crate::map::navigation::NavigationMap>,
) {
    let temp_noise = Fbm::<Perlin>::new(seed.value() + 1);
    let humid_noise = Fbm::<Perlin>::new(seed.value() + 2);

    let width: u32 = 40;
    let height: u32 = 40;
    let half_w = (width / 2).cast_signed();
    let half_h = (height / 2).cast_signed();

    map_data.width = width;
    map_data.height = height;
    map_data.tiles = vec![crate::map::zoning::TileData::default(); (width * height) as usize];

    for x in -half_w..half_w {
        for z in -half_h..half_h {
            let elevation = terrain_gen.get_elevation(x as f32, z as f32);
            let normalized_elevation = (elevation / MAX_HEIGHT).clamp(0.0, 1.0);

            let temp_val =
                ((temp_noise.get([f64::from(x) * 0.05, f64::from(z) * 0.05]) as f32) + 1.0) * 0.5;
            let humid_val =
                ((humid_noise.get([f64::from(x) * 0.05, f64::from(z) * 0.05]) as f32) + 1.0) * 0.5;

            let terrain = get_terrain_from_climate(temp_val, humid_val, normalized_elevation);

            if let Some(tile_data) = map_data.get_tile_mut(x, z) {
                tile_data.terrain = terrain;
                tile_data.elevation = normalized_elevation;
                tile_data.temperature = temp_val;
                tile_data.humidity = humid_val;
                tile_data.roofed = false;
            }
        }
    }

    let mut rng = StdRng::seed_from_u64(u64::from(seed.value()) + 100);
    for x in -half_w..half_w {
        for z in -half_h..half_h {
            if let Some(tile_data) = map_data.get_tile(x, z) {
                // Пещеры стали реже (5% вместо 20%) и только на возвышенностях (> 0.6)
                if tile_data.terrain == TerrainType::Stone
                    && rng.gen_bool(0.05)
                    && tile_data.elevation > 0.6
                {
                    apply_cave_stamp(&mut map_data, x, z);
                }
            }
        }
    }

    for x in -half_w..half_w {
        for z in -half_h..half_h {
            let tile_data = map_data.get_tile(x, z).copied().unwrap_or_default();
            let terrain = tile_data.terrain;

            // Логическая сущность тайла (без меша) для кликов и ИИ
            commands.spawn(zoning::LogicTileBundle {
                transform: Transform::from_xyz(x as f32, 0.0, z as f32),
                tile: Tile { terrain },
                name: Name::new(format!("Tile {x},{z}")),
            });

            let mut cost = crate::map::navigation::COST_BASE;
            if map_data.is_too_steep(x, z) {
                cost = crate::map::navigation::COST_BLOCKER;
            } else {
                match terrain {
                    TerrainType::Water => {
                        cost = crate::map::navigation::COST_BLOCKER;
                    }
                    TerrainType::Mud => {
                        cost = 50;
                    }
                    TerrainType::Stone => {
                        cost = 80;
                    }
                    TerrainType::Grass | TerrainType::Sand | TerrainType::CaveFloor => {}
                }
            }
            nav_map.grid.insert(IVec2::new(x, z), cost);
        }
    }

    // Создаем глобальный ландшафт, воду и крыши одной командой
    commands.queue(crate::economy::mesh_gen::SpawnGlobalTerrainCommand {
        map_data: map_data.clone(),
    });
}

fn apply_cave_stamp(map: &mut MapData, x: i32, z: i32) {
    for dx in -1..=1 {
        for dz in -1..=1 {
            if let Some(tile) = map.get_tile_mut(x + dx, z + dz) {
                tile.terrain = TerrainType::CaveFloor;
                tile.roofed = true;
            }
        }
    }
}

fn get_terrain_from_climate(temp: f32, humid: f32, elev: f32) -> TerrainType {
    if elev < 0.2 {
        return TerrainType::Water;
    }
    if elev < 0.25 {
        return TerrainType::Sand;
    }
    if elev > 0.8 {
        return TerrainType::Stone;
    }

    if humid > 0.7 {
        if temp < 0.3 {
            TerrainType::Mud
        } else {
            TerrainType::Grass
        }
    } else if humid < 0.3 {
        if temp > 0.7 {
            TerrainType::Sand
        } else {
            TerrainType::Grass
        }
    } else {
        TerrainType::Grass
    }
}
