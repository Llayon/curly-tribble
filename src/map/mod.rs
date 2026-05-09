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

pub const MAX_HEIGHT: f32 = 4.0;

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

#[derive(Resource, Default, Clone)]
pub struct MapData {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<crate::map::zoning::TileData>,
}

impl MapData {
    pub fn get_tile(&self, x: i32, z: i32) -> Option<&crate::map::zoning::TileData> {
        let ux = (x + (self.width as i32 / 2)) as u32;
        let uz = (z + (self.height as i32 / 2)) as u32;
        if ux < self.width && uz < self.height {
            Some(&self.tiles[(uz * self.width + ux) as usize])
        } else {
            None
        }
    }

    pub fn get_tile_mut(&mut self, x: i32, z: i32) -> Option<&mut crate::map::zoning::TileData> {
        let ux = (x + (self.width as i32 / 2)) as u32;
        let uz = (z + (self.height as i32 / 2)) as u32;
        if ux < self.width && uz < self.height {
            Some(&mut self.tiles[(uz * self.width + ux) as usize])
        } else {
            None
        }
    }

    pub fn is_too_steep(&self, x: i32, z: i32) -> bool {
        let current_elev = self.get_tile(x, z).map(|t| t.elevation).unwrap_or(0.0);
        for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if let Some(neighbor) = self.get_tile(x + dx, z + dz) {
                if (neighbor.elevation - current_elev).abs() > 0.3 {
                    return true;
                }
            }
        }
        false
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
    let points: Vec<Vec2> = (0..24)
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
            (min_dist / 10.0).min(1.0)
        })
        .collect()
}

fn spawn_map(
    mut commands: Commands,
    assets: Res<crate::economy::GameAssets>,
    seed: Res<WorldSeed>,
    mut map_data: ResMut<MapData>,
    mut nav_map: ResMut<crate::map::navigation::NavigationMap>,
) {
    let elev_noise = Fbm::<Perlin>::new(seed.value());
    let temp_noise = Fbm::<Perlin>::new(seed.value() + 1);
    let humid_noise = Fbm::<Perlin>::new(seed.value() + 2);

    let width = 40;
    let height = 40;
    let half_w = width as i32 / 2;
    let half_h = height as i32 / 2;

    let voronoi_map = generate_voronoi_heights(width, height, seed.value());

    map_data.width = width;
    map_data.height = height;
    map_data.tiles = vec![crate::map::zoning::TileData::default(); (width * height) as usize];

    for x in -half_w..half_w {
        for z in -half_h..half_h {
            let ux = (x + half_w) as u32;
            let uz = (z + half_h) as u32;
            let idx = (uz * width + ux) as usize;
            let voronoi_val = voronoi_map[idx];

            let noise_val = elev_noise.get([x as f64 * 0.05, z as f64 * 0.05]) as f32;
            let temp_val =
                ((temp_noise.get([x as f64 * 0.05, z as f64 * 0.05]) as f32) + 1.0) * 0.5;
            let humid_val =
                ((humid_noise.get([x as f64 * 0.05, z as f64 * 0.05]) as f32) + 1.0) * 0.5;

            let combined_elevation = (voronoi_val + noise_val * 0.2).clamp(0.0, 1.0);
            let terrain = get_terrain_from_climate(temp_val, humid_val, combined_elevation);

            if let Some(tile_data) = map_data.get_tile_mut(x, z) {
                tile_data.terrain = terrain;
                tile_data.elevation = combined_elevation;
                tile_data.temperature = temp_val;
                tile_data.humidity = humid_val;
                tile_data.roofed = false;
            }
        }
    }

    let mut rng = StdRng::seed_from_u64(u64::from(seed.value()) + 100);
    for x in -half_w..half_w {
        for z in -half_h..half_h {
            let terrain = map_data.get_tile(x, z).map(|t| t.terrain);
            if terrain == Some(TerrainType::Stone) && rng.gen_bool(0.2) {
                apply_cave_stamp(&mut map_data, x, z);
            }
        }
    }

    for x in -half_w..half_w {
        for z in -half_h..half_h {
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
                transform: Transform::from_xyz(
                    x as f32,
                    tile_data.elevation * MAX_HEIGHT,
                    z as f32,
                ),
                tile: Tile { terrain },
            });

            let mut cost = crate::map::navigation::COST_BASE;
            match terrain {
                TerrainType::Water => {
                    cost = crate::map::navigation::COST_BLOCKER;
                    entity.insert(NavObstacle { cost });
                }
                TerrainType::Mud => {
                    cost = 50;
                    entity.insert(NavObstacle { cost });
                }
                TerrainType::Stone => {
                    cost = 80;
                    entity.insert(NavObstacle { cost });
                }
                TerrainType::Grass | TerrainType::Sand | TerrainType::CaveFloor => {}
            }

            if map_data.is_too_steep(x, z) {
                cost = crate::map::navigation::COST_BLOCKER;
                entity.insert(NavObstacle { cost });
            }

            nav_map.grid.insert(IVec2::new(x, z), cost);

            if tile_data.roofed {
                commands.spawn(zoning::RoofBundle {
                    mesh: Mesh3d(assets.ground_mesh.clone()),
                    material: MeshMaterial3d(assets.stone_material.clone()),
                    transform: Transform::from_xyz(
                        x as f32,
                        (tile_data.elevation * MAX_HEIGHT) + 1.0,
                        z as f32,
                    ),
                    roof: zoning::Roof,
                });
            }
        }
    }
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
