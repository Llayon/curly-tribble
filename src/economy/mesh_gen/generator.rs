use crate::game_state::{EditorPhase, FactionManager};
use crate::map::data::{OceanState, RoofState};
use crate::map::terrain_gen::TerrainConfig;
use crate::map::{LandscapeFeature, MapData, TerrainType};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

pub struct MeshGeneratorPlugin;

impl Plugin for MeshGeneratorPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayGeometryError {
    MissingFaceForTile(crate::map::HexCoord),
    InvalidSourceFace(crate::map::face_topology::FaceId),
    InvalidSourceVertex(crate::map::face_topology::VertexId),
}

fn extract_warped_face_corners(
    coord: crate::map::HexCoord,
    face_topology: &crate::map::face_topology::types::HexFaceTopology,
) -> Result<[Vec2; 6], OverlayGeometryError> {
    let &face_id = face_topology
        .hex_to_face
        .get(&coord)
        .ok_or(OverlayGeometryError::MissingFaceForTile(coord))?;
    let face = face_topology
        .faces
        .get(face_id.index())
        .ok_or(OverlayGeometryError::InvalidSourceFace(face_id))?;
    let mut corners = [Vec2::ZERO; 6];
    for (i, corner) in corners.iter_mut().enumerate() {
        let v_id = face.vertices[i];
        let v_idx = v_id.index();
        if v_idx >= face_topology.vertices.len() {
            return Err(OverlayGeometryError::InvalidSourceVertex(v_id));
        }
        *corner = face_topology.vertices[v_idx].position;
    }
    Ok(corners)
}

fn append_overlay_face(
    vertices: &mut Vec<[f32; 3]>,
    indices: &mut Vec<u32>,
    corners: &[Vec2; 6],
    y: f32,
    vertex_count: &mut u32,
) {
    let center_xz =
        (corners[0] + corners[1] + corners[2] + corners[3] + corners[4] + corners[5]) / 6.0;
    vertices.push([center_xz.x, y, center_xz.y]);
    for corner in corners {
        vertices.push([corner.x, y, corner.y]);
    }
    let base = *vertex_count;
    for i in 1..=6 {
        let next = if i == 6 { 1 } else { i + 1 };
        indices.extend_from_slice(&[base, base + next, base + i]);
    }
    *vertex_count += 7;
}

#[must_use]
#[allow(clippy::too_many_lines, clippy::missing_panics_doc)]
pub fn create_global_map_meshes(
    map: &MapData,
    topology: &crate::map::topology::TerrainTopology,
    face_topology: &crate::map::face_topology::types::HexFaceTopology,
    phase: EditorPhase,
    faction_manager: &FactionManager,
    config: &TerrainConfig,
) -> (Mesh, Option<Mesh>, Option<Mesh>) {
    let is_flat = phase < EditorPhase::Height3D;
    let is_factions_filter = phase == EditorPhase::Factions;
    let mode = if is_flat {
        crate::map::topology::TerrainHeightMode::Flat
    } else {
        crate::map::topology::TerrainHeightMode::Relief3D
    };

    let heights = crate::map::topology::compute_vertex_heights(topology, map, mode);

    let mut vertices = Vec::with_capacity(topology.vertices_xz.len());
    let mut colors = Vec::with_capacity(topology.vertices_xz.len());

    for (k, pos_xz) in topology.vertices_xz.iter().enumerate() {
        let y = heights[k];
        vertices.push([pos_xz.x, y, pos_xz.y]);

        let influences = &topology.vertex_influences[k];

        // Prefer land cell color if any influencing cell is land
        let land_coord = influences.iter().copied().find(|c| {
            map.get_tile(c.q, c.r)
                .is_some_and(|t| t.ocean_state == OceanState::Land)
        });

        let eval_coord =
            land_coord.unwrap_or_else(|| influences.first().copied().unwrap_or_default());

        let tile_data = map
            .get_tile(eval_coord.q, eval_coord.r)
            .copied()
            .unwrap_or_default();
        let color = tile_color(
            map,
            eval_coord,
            &tile_data,
            phase,
            faction_manager,
            config,
            is_factions_filter,
        );
        colors.push(color);
    }

    let mut indices = Vec::with_capacity(topology.triangles.len() * 3);
    for tri in &topology.triangles {
        indices.push(tri[0]);
        indices.push(tri[1]);
        indices.push(tri[2]);
    }

    let mut water_vertices = Vec::new();
    let mut water_indices = Vec::new();
    let mut roof_vertices = Vec::new();
    let mut roof_indices = Vec::new();

    let mut water_vertex_count = 0;
    let mut roof_vertex_count = 0;

    let mut sorted_coords: Vec<crate::map::HexCoord> = map.tiles.keys().copied().collect();
    sorted_coords.sort_by_key(|c| (c.q, c.r));

    for coord in sorted_coords {
        let tile_data = &map.tiles[&coord];
        let center_y = if is_flat || tile_data.ocean_state == OceanState::Ocean {
            0.0
        } else {
            map.get_hex_height(coord.q, coord.r)
        };

        if (tile_data.landscape_feature == LandscapeFeature::River
            || tile_data.landscape_feature == LandscapeFeature::Lake)
            && tile_data.ocean_state == OceanState::Land
        {
            if let Ok(corners) = extract_warped_face_corners(coord, face_topology) {
                append_overlay_face(
                    &mut water_vertices,
                    &mut water_indices,
                    &corners,
                    center_y,
                    &mut water_vertex_count,
                );
            }
        }

        if tile_data.roof_state == RoofState::Roofed {
            let roof_y = center_y + 2.5;
            if let Ok(corners) = extract_warped_face_corners(coord, face_topology) {
                append_overlay_face(
                    &mut roof_vertices,
                    &mut roof_indices,
                    &corners,
                    roof_y,
                    &mut roof_vertex_count,
                );
            }
        }
    }

    let mut terrain_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    let uvs = vec![[0.5, 0.5]; vertices.len()];
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
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
        return [0.1, 0.4, 0.9, 1.0];
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

fn feature_color(tile: &crate::map::TileData, _phase: EditorPhase) -> [f32; 4] {
    match tile.landscape_feature {
        LandscapeFeature::Mountain => [0.3, 0.25, 0.2, 1.0],
        LandscapeFeature::Lake => [0.4, 0.6, 1.0, 1.0],
        LandscapeFeature::River => [0.0, 0.8, 1.0, 1.0],
        LandscapeFeature::Plateau => [0.5, 0.5, 0.5, 1.0],
        LandscapeFeature::None => match tile.terrain {
            TerrainType::Grass => [0.3, 0.8, 0.2, 1.0],
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
