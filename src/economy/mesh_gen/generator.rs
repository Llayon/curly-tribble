use crate::game_state::{EditorPhase, FactionManager};
use crate::map::data::{OceanState, RoofState};
use crate::map::terrain_gen::TerrainConfig;
use crate::map::{LandscapeFeature, MapData, TerrainType, HEX_SIZE, MAX_HEIGHT};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

pub struct MeshGeneratorPlugin;

impl Plugin for MeshGeneratorPlugin {
    fn build(&self, _app: &mut App) {}
}

#[must_use]
pub fn create_global_map_meshes(
    map: &MapData,
    phase: EditorPhase,
    faction_manager: &FactionManager,
    config: &TerrainConfig,
) -> (Mesh, Option<Mesh>, Option<Mesh>) {
    let is_flat = phase != EditorPhase::Height3D;
    let is_factions_filter = phase == EditorPhase::Factions;

    let mut vertices = Vec::new();
    let mut colors = Vec::new();
    let mut indices = Vec::new();
    let mut water_vertices = Vec::new();
    let mut water_indices = Vec::new();
    let mut roof_vertices = Vec::new();
    let mut roof_indices = Vec::new();

    let size = HEX_SIZE;
    let mut vertex_count = 0;
    let mut water_vertex_count = 0;
    let mut roof_vertex_count = 0;

    for (&coord, tile_data) in &map.tiles {
        let center_world = coord.to_world(size);
        let center_y = if is_flat || tile_data.ocean_state == OceanState::Ocean {
            0.0
        } else {
            tile_data.elevation * MAX_HEIGHT
        };

        let color = tile_color(
            map,
            coord,
            tile_data,
            phase,
            faction_manager,
            config,
            is_factions_filter,
        );

        vertices.push([center_world.x, center_y, center_world.z]);
        colors.push(color);
        for i in 0..6 {
            let angle_deg = 60.0 * i as f32 + 30.0;
            let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
            let vx = center_world.x + size * angle_rad.cos();
            let vz = center_world.z + size * angle_rad.sin();
            vertices.push([vx, center_y, vz]);
            colors.push(color);
        }
        let base = vertex_count;
        for i in 1..=6 {
            let next = if i == 6 { 1 } else { i + 1 };
            indices.extend_from_slice(&[base, base + next, base + i]);
        }
        vertex_count += 7;

        if (tile_data.landscape_feature == LandscapeFeature::River
            || tile_data.landscape_feature == LandscapeFeature::Lake)
            && tile_data.ocean_state == OceanState::Land
        {
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

        if tile_data.roof_state == RoofState::Roofed {
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

    let mut terrain_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    terrain_mesh.insert_indices(Indices::U32(indices));
    terrain_mesh.compute_normals();

    let water_mesh = create_optional_mesh(water_vertices, water_indices);
    let roof_mesh = create_optional_mesh(roof_vertices, roof_indices);

    (terrain_mesh, water_mesh, roof_mesh)
}

fn tile_color(
    map: &MapData,
    coord: crate::map::HexCoord,
    tile: &crate::map::TileData,
    phase: EditorPhase,
    factions: &FactionManager,
    config: &TerrainConfig,
    faction_filter: bool,
) -> [f32; 4] {
    if phase == EditorPhase::Sediments
        && config.build_area_layer.is_visible()
        && tile.ocean_state == OceanState::Land
        && !map.is_too_steep(coord.q, coord.r)
        && tile.terrain.allows_buildings()
    {
        return [0.2, 1.0, 0.2, 1.0];
    }
    if tile.ocean_state == OceanState::Ocean {
        return [0.0, 0.2, 0.5, 1.0];
    }
    let base = feature_color(tile, phase);
    if !config.faction_layer.is_visible() {
        return base;
    }
    let Some(faction_id) = tile.faction_id else {
        return base;
    };
    let Some(faction) = factions.factions.iter().find(|f| f.id == faction_id) else {
        return base;
    };
    let color = faction.color.to_linear().to_f32_array();
    if faction_filter {
        return [color[0], color[1], color[2], 1.0];
    }
    let mix = 0.3;
    [
        base[0] * (1.0 - mix) + color[0] * mix,
        base[1] * (1.0 - mix) + color[1] * mix,
        base[2] * (1.0 - mix) + color[2] * mix,
        1.0,
    ]
}

fn feature_color(tile: &crate::map::TileData, phase: EditorPhase) -> [f32; 4] {
    match tile.landscape_feature {
        LandscapeFeature::Mountain => [0.3, 0.25, 0.2, 1.0],
        LandscapeFeature::Lake => [0.4, 0.6, 1.0, 1.0],
        LandscapeFeature::River => [0.0, 0.8, 1.0, 1.0],
        LandscapeFeature::Plateau => [0.5, 0.5, 0.5, 1.0],
        LandscapeFeature::None if phase < EditorPhase::Sediments => [0.15, 0.15, 0.18, 1.0],
        LandscapeFeature::None => match tile.terrain {
            TerrainType::Grass => [0.2, 0.5, 0.1, 1.0],
            TerrainType::Dirt => [0.4, 0.3, 0.2, 1.0],
            TerrainType::Dusty => [0.6, 0.5, 0.4, 1.0],
            TerrainType::Fertile => [0.1, 0.4, 0.05, 1.0],
            TerrainType::Mossy => [0.3, 0.4, 0.1, 1.0],
            TerrainType::Steppe => [0.5, 0.6, 0.3, 1.0],
            TerrainType::Stony => [0.4, 0.4, 0.45, 1.0],
            TerrainType::Swamp => [0.2, 0.25, 0.2, 1.0],
        },
    }
}

fn create_optional_mesh(vertices: Vec<[f32; 3]>, indices: Vec<u32>) -> Option<Mesh> {
    if vertices.is_empty() {
        return None;
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_indices(Indices::U32(indices));
    mesh.compute_normals();
    Some(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{HexCoord, TileData};

    #[test]
    fn omits_empty_overlay_meshes() {
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());

        let (_terrain, water, roof) = create_global_map_meshes(
            &map,
            EditorPhase::Shape,
            &FactionManager::default(),
            &TerrainConfig::default(),
        );

        assert!(water.is_none());
        assert!(roof.is_none());
    }
}
