use crate::sets::StartupSet;
use bevy::prelude::*;
use construction::ConstructionPlugin;
use navigation::{NavObstacle, NavigationPlugin};
use noise::{Fbm, NoiseFn, Perlin};
use rand::prelude::*;
use resources::ResourcesPlugin;
use zoning::{TerrainType, Tile};

pub mod atmosphere;
pub mod construction;
pub mod navigation;
pub mod resources;
pub mod visibility;
pub mod zoning;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldSeed>()
            .init_resource::<MapData>()
            .add_plugins((
                zoning::ZoningPlugin,
                ResourcesPlugin,
                ConstructionPlugin,
                NavigationPlugin,
                visibility::VisibilityPlugin,
            ))
            .add_systems(Startup, spawn_map.in_set(StartupSet::SpawnEntities));
    }
}

#[derive(Resource)]
pub struct WorldSeed(u32);

impl WorldSeed {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    pub fn value(&self) -> u32 {
        self.0
    }
}

impl Default for WorldSeed {
    fn default() -> Self {
        Self(42) // Тот самый сид
    }
}

#[derive(Resource, Default)]
pub struct MapData {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<crate::map::zoning::TileData>,
}

impl MapData {
    pub fn get_tile(&self, x: u32, z: u32) -> Option<&crate::map::zoning::TileData> {
        if x < self.width && z < self.height {
            Some(&self.tiles[(z * self.width + x) as usize])
        } else {
            None
        }
    }

    pub fn get_tile_mut(&mut self, x: u32, z: u32) -> Option<&mut crate::map::zoning::TileData> {
        if x < self.width && z < self.height {
            Some(&mut self.tiles[(z * self.width + x) as usize])
        } else {
            None
        }
    }
}

#[derive(Bundle)]
pub struct MapTileBundle {
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub tile: Tile,
}

fn generate_voronoi_heights(width: u32, height: u32, seed: u32) -> Vec<f32> {
    let mut rng = rand::prelude::StdRng::seed_from_u64(u64::from(seed));
    let points: Vec<Vec2> = (0..12)
        .map(|_| {
            Vec2::new(
                rng.gen_range(0.0..width as f32),
                rng.gen_range(0.0..height as f32),
            )
        })
        .collect();

    (0..width * height)
        .map(|i| {
            let x = (i % width) as f32;
            let z = (i / width) as f32;
            let mut min_dist = f32::MAX;
            for p in &points {
                min_dist = min_dist.min(p.distance(Vec2::new(x, z)));
            }
            // Normalize: min_dist / 10.0 or similar.
            // Using 10.0 as a base "radius" for features.
            (min_dist / 10.0).min(1.0)
        })
        .collect()
}

fn spawn_map(
    mut commands: Commands,
    assets: Res<crate::economy::GameAssets>,
    seed: Res<WorldSeed>,
    mut map_data: ResMut<MapData>,
) {
    let elev_noise = Fbm::<Perlin>::new(seed.value());
    let temp_noise = Fbm::<Perlin>::new(seed.value() + 1);
    let humid_noise = Fbm::<Perlin>::new(seed.value() + 2);

    let width = 20;
    let height = 20;

    let voronoi_map = generate_voronoi_heights(width, height, seed.value());

    map_data.width = width;
    map_data.height = height;
    map_data.tiles = vec![crate::map::zoning::TileData::default(); (width * height) as usize];

    for x in 0..width {
        for z in 0..height {
            let idx = (z * width + x) as usize;
            let voronoi_val = voronoi_map[idx];

            // Генерируем значения шума
            let noise_val = elev_noise.get([x as f64 * 0.2, z as f64 * 0.2]) as f32;
            let temp_val = ((temp_noise.get([x as f64 * 0.1, z as f64 * 0.1]) as f32) + 1.0) * 0.5;
            let humid_val =
                ((humid_noise.get([x as f64 * 0.1, z as f64 * 0.1]) as f32) + 1.0) * 0.5;

            // Combine Voronoi and Noise (hills on top of skeleton)
            let combined_elevation = (voronoi_val + noise_val * 0.2).clamp(0.0, 1.0);

            let terrain = get_terrain_from_climate(temp_val, humid_val, combined_elevation);

            // Update MapData
            if let Some(tile_data) = map_data.get_tile_mut(x, z) {
                tile_data.terrain = terrain;
                tile_data.elevation = combined_elevation;
                tile_data.temperature = temp_val;
                tile_data.humidity = humid_val;
                tile_data.roofed = false;
            }
        }
    }

    // Apply cave stamps
    let mut rng = StdRng::seed_from_u64(u64::from(seed.value()) + 100);
    for x in 0..width {
        for z in 0..height {
            let terrain = map_data.get_tile(x, z).map(|t| t.terrain);
            if terrain == Some(TerrainType::Stone) && rng.gen_bool(0.2) {
                apply_cave_stamp(&mut map_data, x as i32, z as i32);
            }
        }
    }

    // Actual spawning
    for x in 0..width {
        for z in 0..height {
            let tile_data = map_data.get_tile(x, z).cloned().unwrap_or_default();
            let terrain = tile_data.terrain;

            let material = match terrain {
                TerrainType::Grass => assets.ground_material.clone(),
                TerrainType::Mud => assets.mud_material.clone(),
                TerrainType::Water => assets.water_material.clone(),
                TerrainType::Stone => assets.stone_material.clone(),
                TerrainType::Sand => assets.sand_material.clone(),
                TerrainType::CaveFloor => assets.ground_material.clone(),
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
                TerrainType::Stone => {
                    entity.insert(NavObstacle { cost: 80 }); // Труднопроходимо
                }
                TerrainType::Grass | TerrainType::Sand | TerrainType::CaveFloor => {} // Базовая стоимость 20
            }

            if tile_data.roofed {
                commands.spawn(zoning::RoofBundle {
                    mesh: Mesh3d(assets.ground_mesh.clone()),
                    material: MeshMaterial3d(assets.stone_material.clone()),
                    transform: Transform::from_xyz(x as f32, 1.0, z as f32),
                    roof: zoning::Roof,
                });
            }
        }
    }
}

fn apply_cave_stamp(map: &mut MapData, x: i32, z: i32) {
    for dx in -1..=1 {
        for dz in -1..=1 {
            let tx = x + dx;
            let tz = z + dz;
            if tx >= 0 && tz >= 0 {
                if let Some(tile) = map.get_tile_mut(tx as u32, tz as u32) {
                    tile.terrain = TerrainType::CaveFloor;
                    tile.roofed = true;
                }
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

    // Simple Whittaker matrix
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
