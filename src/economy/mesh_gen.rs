use crate::game_state::EditorPhase;
use crate::map::zoning::{
    GlobalTerrainBundle, MapData, Roof, TerrainType, WaterBundle,
};
use crate::map::MapEntity;
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

pub struct MeshGenPlugin;

impl Plugin for MeshGenPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            draw_hex_grid_gizmos.run_if(not(in_state(EditorPhase::Height3D))),
        );
    }
}

pub struct SpawnGlobalTerrainCommand {
    pub map_data: MapData,
    pub phase: EditorPhase,
}

impl Command for SpawnGlobalTerrainCommand {
    fn apply(self, world: &mut World) {
        let is_flat = self.phase != EditorPhase::Height3D;

        let (mesh, water_mesh, roof_mesh) = create_global_map_meshes(&self.map_data, is_flat);

        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let terrain_handle = meshes.add(mesh);
        let water_handle = meshes.add(water_mesh);
        let roof_handle = meshes.add(roof_mesh);

        let assets = world.resource::<crate::economy::GameAssets>();
        let ground_mat = assets.ground_material.clone();
        let water_mat = assets.water_material.clone();
        let mountain_mat = assets.mountain_material.clone();

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        if let Some(mat) = materials.get_mut(&ground_mat) {
            mat.unlit = is_flat;
        }
        if let Some(mat) = materials.get_mut(&water_mat) {
            mat.unlit = is_flat;
        }

        // Спавним основной ландшафт
        world.spawn(GlobalTerrainBundle {
            mesh: Mesh3d(terrain_handle),
            material: MeshMaterial3d(ground_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            name: Name::new("Global Terrain"),
            marker: MapEntity,
        });

        // Спавним воду
        world.spawn(WaterBundle {
            mesh: Mesh3d(water_handle),
            material: MeshMaterial3d(water_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            name: Name::new("Water Layer"),
            marker: MapEntity,
        });

        // Спавним крыши пещер (единым мешем)
        world.spawn(crate::map::zoning::MountainRoofBundle {
            mesh: Mesh3d(roof_handle),
            material: MeshMaterial3d(mountain_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            roof: Roof,
            name: Name::new("Global Mountain Roofs"),
            marker: MapEntity,
        });
    }
}

#[must_use]
pub fn create_global_map_meshes(map: &MapData, is_flat: bool) -> (Mesh, Mesh, Mesh) {
    debug!(
        "MESH_GEN: Starting global mesh generation for {} tiles (Flat: {}).",
        map.tiles.len(),
        is_flat
    );
    let mut vertices = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();

    let mut water_vertices = Vec::new();
    let mut water_indices = Vec::new();

    let mut roof_vertices = Vec::new();
    let mut roof_indices = Vec::new();

    let size = crate::map::zoning::HEX_SIZE;
    let mut vertex_count = 0;
    let mut water_vertex_count = 0;
    let mut roof_vertex_count = 0;

    for (&coord, tile_data) in &map.tiles {
        let center_world = coord.to_world(size);
        let center_y = if is_flat || tile_data.is_ocean {
            0.0
        } else {
            tile_data.elevation * crate::map::zoning::MAX_HEIGHT
        };
        let color = if tile_data.is_ocean {
            [0.02, 0.05, 0.3, 1.0] // Deep Ocean Blue
        } else {
            match tile_data.terrain {
                TerrainType::Grass => [0.2, 0.5, 0.1, 1.0],
                TerrainType::Mud => [0.3, 0.2, 0.1, 1.0],
                TerrainType::Sand => [0.8, 0.7, 0.3, 1.0],
                TerrainType::Stone => [0.4, 0.4, 0.4, 1.0],
                TerrainType::Water => [0.1, 0.2, 0.5, 1.0],
                TerrainType::CaveFloor => [0.1, 0.1, 0.1, 1.0],
            }
        };

        // Center vertex
        vertices.push([center_world.x, center_y, center_world.z]);
        colors.push(color);

        // 6 Corner vertices
        for i in 0..6 {
            let angle_deg = 60.0 * i as f32 + 30.0;
            let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
            let vx = center_world.x + size * angle_rad.cos();
            let vz = center_world.z + size * angle_rad.sin();

            vertices.push([vx, center_y, vz]);
            colors.push(color);
        }

        // 6 Triangles
        let base = vertex_count;
        for i in 1..=6 {
            let next = if i == 6 { 1 } else { i + 1 };
            indices.extend_from_slice(&[base, base + next, base + i]);
        }
        vertex_count += 7;

        // --- WATER LAYER ---
        if tile_data.terrain == TerrainType::Water {
            water_vertices.push([center_world.x, center_y, center_world.z]);
            for i in 0..6 {
                let angle_deg = 60.0 * i as f32 + 30.0;
                let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
                let vx = center_world.x + size * angle_rad.cos();
                let vz = center_world.z + size * angle_rad.sin();
                water_vertices.push([vx, center_y, vz]);
            }
            let base_w = water_vertex_count;
            for i in 1..=6 {
                let next = if i == 6 { 1 } else { i + 1 };
                water_indices.extend_from_slice(&[base_w, base_w + next, base_w + i]);
            }
            water_vertex_count += 7;
        }

        // --- ROOF LAYER ---
        if tile_data.roofed {
            let roof_y = center_y + 2.5;
            roof_vertices.push([center_world.x, roof_y, center_world.z]);
            for i in 0..6 {
                let angle_deg = 60.0 * i as f32 + 30.0;
                let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
                let vx = center_world.x + size * angle_rad.cos();
                let vz = center_world.z + size * angle_rad.sin();
                roof_vertices.push([vx, roof_y, vz]);
            }
            let base_r = roof_vertex_count;
            for i in 1..=6 {
                let next = if i == 6 { 1 } else { i + 1 };
                roof_indices.extend_from_slice(&[base_r, base_r + next, base_r + i]);
            }
            roof_vertex_count += 7;
        }
    }

    debug!("MESH_GEN: Generated {} vertices for terrain.", vertices.len());

    let mut terrain_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    terrain_mesh.insert_indices(Indices::U32(indices));
    terrain_mesh.compute_normals();

    let mut water_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    water_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, water_vertices);
    water_mesh.insert_indices(Indices::U32(water_indices));
    water_mesh.compute_normals();

    let mut roof_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    roof_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, roof_vertices);
    roof_mesh.insert_indices(Indices::U32(roof_indices));
    roof_mesh.compute_normals();

    (terrain_mesh, water_mesh, roof_mesh)
}

fn draw_hex_grid_gizmos(mut gizmos: Gizmos, map: Res<MapData>) {
    let size = crate::map::zoning::HEX_SIZE;
    let color = Color::BLACK;
    let y = 0.01;

    for &coord in map.tiles.keys() {
        let center = coord.to_world(size);
        let mut points = Vec::new();

        for i in 0..6 {
            let angle_deg = 60.0 * i as f32 + 30.0;
            let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
            let vx = center.x + size * angle_rad.cos();
            let vz = center.z + size * angle_rad.sin();
            points.push(Vec3::new(vx, y, vz));
        }
        // Замыкаем контур
        let first = points[0];
        points.push(first);

        gizmos.linestrip(points, color);
    }
}
