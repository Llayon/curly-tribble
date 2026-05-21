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

#[derive(Resource, Default)]
pub struct GeneratedMapAssets {
    pub terrain: Option<Handle<Mesh>>,
    pub water: Option<Handle<Mesh>>,
    pub roof: Option<Handle<Mesh>>,
}

impl Plugin for MeshGenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GeneratedMapAssets>()
            .add_systems(
                Update,
                (
                    draw_hex_grid_gizmos.run_if(not(in_state(EditorPhase::Height3D))),
                    draw_factions_gizmos.run_if(in_state(EditorPhase::Factions)),
                    draw_cliffs_gizmos.run_if(in_state(EditorPhase::Landscape)),
                    draw_forest_gizmos.run_if(in_state(EditorPhase::Sediments)),
                ),
            );
    }
}

pub struct SpawnGlobalTerrainCommand {
    pub map_data: MapData,
    pub phase: EditorPhase,
    pub faction_manager: crate::game_state::FactionManager,
    pub config: crate::map::terrain_gen::TerrainConfig,
}

impl Command for SpawnGlobalTerrainCommand {
    fn apply(self, world: &mut World) {
        let is_flat = self.phase != EditorPhase::Height3D;

        // 1. Очистка старых ресурсов из VRAM
        let old_handles = if let Some(mut gen_assets) = world.get_resource_mut::<GeneratedMapAssets>() {
            (gen_assets.terrain.take(), gen_assets.water.take(), gen_assets.roof.take())
        } else {
            (None, None, None)
        };

        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        if let Some(h) = old_handles.0 { meshes.remove(&h); }
        if let Some(h) = old_handles.1 { meshes.remove(&h); }
        if let Some(h) = old_handles.2 { meshes.remove(&h); }

        // 2. Генерация новых мешей
        let (mesh, water_mesh, roof_mesh) = create_global_map_meshes(&self.map_data, self.phase, &self.faction_manager, &self.config);
        
        // Повторно берем meshes т.к. remove мог его дропнуть (но Command имеет &mut World)
        let terrain_handle = meshes.add(mesh);
        let water_handle = meshes.add(water_mesh);
        let roof_handle = meshes.add(roof_mesh);

        // 3. Сохранение новых хэндлов для будущего удаления
        if let Some(mut gen_assets) = world.get_resource_mut::<GeneratedMapAssets>() {
            gen_assets.terrain = Some(terrain_handle.clone());
            gen_assets.water = Some(water_handle.clone());
            gen_assets.roof = Some(roof_handle.clone());
        }

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

        // 4. Спавн сущностей с новыми мешами
        world.spawn(GlobalTerrainBundle {
            mesh: Mesh3d(terrain_handle),
            material: MeshMaterial3d(ground_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            name: Name::new("Global Terrain"),
            marker: MapEntity,
        });

        world.spawn(WaterBundle {
            mesh: Mesh3d(water_handle),
            material: MeshMaterial3d(water_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            name: Name::new("Water Layer"),
            marker: MapEntity,
        });

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
pub fn create_global_map_meshes(
    map: &MapData,
    phase: EditorPhase,
    faction_manager: &crate::game_state::FactionManager,
    config: &crate::map::terrain_gen::TerrainConfig,
) -> (Mesh, Mesh, Mesh) {
    let is_flat = phase != EditorPhase::Height3D;
    let is_factions_filter = phase == EditorPhase::Factions;

    debug!(
        "MESH_GEN: Starting global mesh generation for {} tiles (Flat: {}, FactionsFilter: {}).",
        map.tiles.len(),
        is_flat,
        is_factions_filter
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
        
        let mut color = if tile_data.is_ocean {
            [0.02, 0.05, 0.3, 1.0] // Deep Ocean Blue
        } else if phase == EditorPhase::Landscape {
            match tile_data.landscape_feature {
                crate::map::zoning::LandscapeFeature::Mountain => [0.3, 0.25, 0.2, 1.0], // Dark Brown/Rock
                crate::map::zoning::LandscapeFeature::Lake => [0.0, 0.6, 0.8, 1.0],     // Freshwater Blue
                crate::map::zoning::LandscapeFeature::River => [0.1, 0.3, 0.7, 1.0],    // Deep River Blue
                crate::map::zoning::LandscapeFeature::Plateau => [0.5, 0.5, 0.5, 1.0],  // Stone Gray
                crate::map::zoning::LandscapeFeature::None => [0.15, 0.15, 0.18, 1.0], // Dark Gray base
            }
        } else if phase == EditorPhase::Sediments {
            match tile_data.terrain {
                TerrainType::Grass => [0.2, 0.5, 0.1, 1.0],   // Green
                TerrainType::Dirt => [0.4, 0.3, 0.2, 1.0],    // Brown
                TerrainType::Dusty => [0.6, 0.5, 0.4, 1.0],   // Dusty Gray/Brown
                TerrainType::Fertile => [0.1, 0.4, 0.05, 1.0], // Darker Green
                TerrainType::Mossy => [0.3, 0.4, 0.1, 1.0],   // Yellowish Green
                TerrainType::Steppe => [0.5, 0.6, 0.3, 1.0],  // Dry Green/Gray
                TerrainType::Stony => [0.4, 0.4, 0.45, 1.0],  // Grayish
                TerrainType::Swamp => [0.2, 0.25, 0.2, 1.0],  // Dark Swampy
            }
        } else if is_factions_filter {
            if let Some(f_id) = tile_data.faction_id {
                let faction = faction_manager.factions.iter().find(|f| f.id == f_id);
                if let Some(f) = faction {
                    let c = f.color.to_linear().to_f32_array();
                    [c[0], c[1], c[2], 1.0]
                } else {
                    [0.15, 0.15, 0.18, 1.0] // Dark Gray fallback
                }
            } else {
                [0.15, 0.15, 0.18, 1.0] // Dark Gray for Factions Phase
            }
        } else {
            match tile_data.terrain {
                TerrainType::Grass => [0.2, 0.5, 0.1, 1.0],
                TerrainType::Dirt => [0.4, 0.3, 0.2, 1.0],
                TerrainType::Dusty => [0.6, 0.5, 0.4, 1.0],
                TerrainType::Fertile => [0.1, 0.4, 0.05, 1.0],
                TerrainType::Mossy => [0.3, 0.4, 0.1, 1.0],
                TerrainType::Steppe => [0.5, 0.6, 0.3, 1.0],
                TerrainType::Stony => [0.4, 0.4, 0.45, 1.0],
                TerrainType::Swamp => [0.2, 0.25, 0.2, 1.0],
            }
        };

        // Фильтр "Show Build Area" (только в Фазе 4)
        if phase == EditorPhase::Sediments && config.show_build_area && !tile_data.is_ocean {
            let is_flat = !map.is_too_steep(coord.q, coord.r);
            let allows_buildings = tile_data.terrain.traits().allow_buildings;
            if is_flat && allows_buildings {
                color = [0.2, 1.0, 0.2, 1.0]; // Bright Green
            }
        }

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
        if (tile_data.landscape_feature == crate::map::zoning::LandscapeFeature::River || tile_data.landscape_feature == crate::map::zoning::LandscapeFeature::Lake) && !tile_data.is_ocean {
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

fn draw_cliffs_gizmos(mut gizmos: Gizmos, map_data: Res<MapData>) {
    let size = crate::map::zoning::HEX_SIZE;
    let y = 0.1;

    for (edge, data) in &map_data.edges {
        if data.is_cliff {
            let center_a = edge.a.to_world(size);
            let center_b = edge.b.to_world(size);
            
            let between = center_b - center_a;
            let dist = between.length();
            if dist < 0.001 { continue; }
            
            let dir = between / dist;
            let perp = Vec3::new(-dir.z, 0.0, dir.x); 
            
            let midpoint = (center_a + center_b) * 0.5;
            let edge_half_len = size * 0.48; // Чуть короче, чтобы не сливались
            
            let start = midpoint - perp * edge_half_len;
            let end = midpoint + perp * edge_half_len;
            
            // Основная линия клиффа (белая)
            gizmos.line(start + Vec3::Y * y, end + Vec3::Y * y, Color::WHITE);
            
            // Маленькая стрелочка направления (тоже белая)
            let arrow_dir = if data.direction { dir } else { -dir };
            let arrow_base = midpoint + arrow_dir * 0.15;
            let arrow_tip = midpoint + arrow_dir * 0.35;
            
            // Рисуем маленькую "галочку" или линию
            gizmos.line(midpoint + Vec3::Y * y, arrow_tip + Vec3::Y * y, Color::WHITE);
            
            // Боковые усики стрелочки для "остроты"
            let arrow_perp = Vec3::new(-arrow_dir.z, 0.0, arrow_dir.x) * 0.1;
            gizmos.line(arrow_tip + Vec3::Y * y, (arrow_base + arrow_perp) + Vec3::Y * y, Color::WHITE);
            gizmos.line(arrow_tip + Vec3::Y * y, (arrow_base - arrow_perp) + Vec3::Y * y, Color::WHITE);
        }
    }
}

fn draw_factions_gizmos(
    mut _gizmos: Gizmos,
    _map_data: Res<MapData>,
    _faction_manager: Res<crate::game_state::FactionManager>,
) {
    // Solid fill is now handled via vertex colors in the mesh generator.
    // Wireframe hexagons removed for clarity.
}

fn draw_hex_grid_gizmos(mut gizmos: Gizmos, map: Res<MapData>) {
    let size = crate::map::zoning::HEX_SIZE;
    let color = Color::BLACK;
    let y = 0.02;

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

fn draw_forest_gizmos(mut gizmos: Gizmos, map_data: Res<MapData>) {
    let size = crate::map::zoning::HEX_SIZE;
    let y = 0.1;

    for (coord, tile) in &map_data.tiles {
        if tile.forest_type != crate::map::zoning::ForestType::None && tile.forest_density > 0.0 {
            let center = coord.to_world(size);
            let color = match tile.forest_type {
                crate::map::zoning::ForestType::Deciduous => Color::srgb(0.0, 0.8, 0.2),
                crate::map::zoning::ForestType::Coniferous => Color::srgb(0.0, 0.4, 0.1),
                _ => Color::NONE,
            };

            let density_count = (tile.forest_density * 5.0) as i32 + 1;
            for i in 0..density_count {
                let offset_x = (i as f32 * 1.3).cos() * 0.3;
                let offset_z = (i as f32 * 1.3).sin() * 0.3;
                let pos = center + Vec3::new(offset_x, 0.0, offset_z);
                gizmos.line(pos + Vec3::Y * y, pos + Vec3::Y * (y + 0.4), color);
            }
        }
    }
}
