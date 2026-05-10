use crate::map::zoning::{
    GlobalTerrainBundle, MapData, Roof, SmoothTileBundle, TerrainType, Tile, TileLayer, WaterBundle,
};
use bevy::prelude::*;

pub struct MeshGenPlugin;

impl Plugin for MeshGenPlugin {
    fn build(&self, _app: &mut App) {}
}

pub struct SpawnGlobalTerrainCommand {
    pub map_data: MapData,
}

impl Command for SpawnGlobalTerrainCommand {
    fn apply(self, world: &mut World) {
        let (mesh, water_mesh) = create_global_map_meshes(&self.map_data);

        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let terrain_handle = meshes.add(mesh);
        let water_handle = meshes.add(water_mesh);

        let assets = world.resource::<crate::economy::GameAssets>();
        let ground_mat = assets.ground_material.clone();
        let water_mat = assets.water_material.clone();

        // Спавним основной ландшафт
        world.spawn(GlobalTerrainBundle {
            mesh: Mesh3d(terrain_handle),
            material: MeshMaterial3d(ground_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            name: Name::new("Global Terrain"),
        });

        // Спавним воду
        world.spawn(WaterBundle {
            mesh: Mesh3d(water_handle),
            material: MeshMaterial3d(water_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            name: Name::new("Water Layer"),
        });
    }
}

pub struct SpawnSmoothTileCommand {
    pub x: i32,
    pub z: i32,
    pub h_nw: f32,
    pub h_ne: f32,
    pub h_sw: f32,
    pub h_se: f32,
    pub offset_y: f32,
    pub material: Handle<StandardMaterial>,
    pub terrain: TerrainType,
    pub layer: TileLayer,
}

impl Command for SpawnSmoothTileCommand {
    fn apply(self, world: &mut World) {
        let mesh = create_smooth_tile_mesh(self.h_nw, self.h_ne, self.h_sw, self.h_se);
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh_handle = meshes.add(mesh);

        let mut entity = world.spawn(SmoothTileBundle {
            mesh: Mesh3d(mesh_handle),
            material: MeshMaterial3d(self.material),
            transform: Transform::from_xyz(self.x as f32, self.offset_y, self.z as f32),
            tile: Tile {
                terrain: self.terrain,
            },
        });

        match self.layer {
            TileLayer::Roof => {
                entity.insert(Roof);
            }
            TileLayer::Ground => {}
        }
    }
}

pub fn create_global_map_meshes(map: &MapData) -> (Mesh, Mesh) {
    let width = map.width;
    let height = map.height;
    let half_w = width as i32 / 2;
    let half_h = height as i32 / 2;

    // ВЕРШИНЫ И ЦВЕТА
    let mut vertices = Vec::new();
    let mut colors = Vec::new();

    for z in -half_h..=half_h {
        for x in -half_w..=half_w {
            let h = map.get_corner_height(x, z);
            vertices.push([x as f32, h, z as f32]);

            let tile = map.get_tile(x, z).or_else(|| map.get_tile(x - 1, z - 1));
            let color = match tile.map(|t| t.terrain).unwrap_or(TerrainType::Grass) {
                TerrainType::Grass => [0.2, 0.5, 0.1, 1.0],
                TerrainType::Mud => [0.3, 0.2, 0.1, 1.0],
                TerrainType::Sand => [0.8, 0.7, 0.3, 1.0],
                TerrainType::Stone => [0.4, 0.4, 0.4, 1.0],
                TerrainType::Water => [0.1, 0.2, 0.5, 1.0],
                TerrainType::CaveFloor => [0.1, 0.1, 0.1, 1.0],
            };
            colors.push(color);
        }
    }

    // ИНДЕКСЫ
    let mut indices = Vec::new();
    let row_size = width + 1;
    for z in 0..height {
        for x in 0..width {
            let nw = z * row_size + x;
            let ne = nw + 1;
            let sw = (z + 1) * row_size + x;
            let se = sw + 1;

            indices.push(nw);
            indices.push(se);
            indices.push(ne);

            indices.push(nw);
            indices.push(sw);
            indices.push(se);
        }
    }

    // В Bevy 0.18.1 типы меша доступны через модули рендеринга
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::Indices;
    use bevy::render::render_resource::PrimitiveTopology;

    let mut terrain_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    terrain_mesh.insert_indices(Indices::U32(indices));
    terrain_mesh.compute_normals();

    let water_level = 0.2 * crate::map::zoning::MAX_HEIGHT;
    let w_size_x = width as f32;
    let w_size_z = height as f32;

    let mut water_mesh = Mesh::from(Plane3d::new(Vec3::Y, Vec2::new(w_size_x, w_size_z)));
    let water_vertices = vec![
        [-w_size_x / 2.0, water_level, -w_size_z / 2.0],
        [w_size_x / 2.0, water_level, -w_size_z / 2.0],
        [-w_size_x / 2.0, water_level, w_size_z / 2.0],
        [w_size_x / 2.0, water_level, w_size_z / 2.0],
    ];
    water_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, water_vertices);

    (terrain_mesh, water_mesh)
}

pub fn create_smooth_tile_mesh(h_nw: f32, h_ne: f32, h_sw: f32, h_se: f32) -> Mesh {
    let mut mesh = Mesh::from(Plane3d::default());
    let vertices = vec![
        [-0.5, h_nw, -0.5],
        [0.5, h_ne, -0.5],
        [-0.5, h_sw, 0.5],
        [0.5, h_se, 0.5],
    ];
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();
    mesh
}
