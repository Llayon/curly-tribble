// src/economy/mesh_gen/generator.rs
//! Legacy topology-driven ground mesh generation, tile coloring, and the
//! shared `OverlayGeometryError`. The M5.1 bake pipeline lives in `bake.rs`
//! and the water/roof overlays live in `overlay.rs`.

use crate::game_state::{EditorPhase, FactionManager};
use crate::map::data::OceanState;
use crate::map::height_graph::types::HeightNodeId;
use crate::map::terrain_gen::TerrainConfig;
use crate::map::{LandscapeFeature, MapData, TerrainType};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

pub use super::bake::create_global_map_meshes_from_bake;
pub(crate) use super::overlay::build_water_and_roof_meshes;

pub struct MeshGeneratorPlugin;

impl Plugin for MeshGeneratorPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlayGeometryError {
    MissingFaceForTile(crate::map::HexCoord),
    InvalidSourceFace(crate::map::face_topology::FaceId),
    InvalidSourceVertex(crate::map::face_topology::VertexId),
    InvalidHeightNode(HeightNodeId),
    HeightNodeIndexOverflow(HeightNodeId),
    MissingBakeVertexOwner(HeightNodeId),
}

/// Creates map ground, water, and roof meshes from authoritative topology.
///
/// # Errors
/// Returns `OverlayGeometryError` if any tile's face or vertex lookup fails in `HexFaceTopology`.
#[allow(clippy::too_many_arguments)]
pub fn create_global_map_meshes(
    map: &MapData,
    topology: &crate::map::topology::TerrainTopology,
    face_topology: &crate::map::face_topology::types::HexFaceTopology,
    phase: EditorPhase,
    faction_manager: &FactionManager,
    config: &TerrainConfig,
) -> Result<(Mesh, Option<Mesh>, Option<Mesh>), OverlayGeometryError> {
    let flat_surface = phase < EditorPhase::Height3D;
    let faction_filter = phase == EditorPhase::Factions;
    let mode = if flat_surface {
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
            faction_filter,
        );
        colors.push(color);
    }

    let mut indices = Vec::with_capacity(topology.triangles.len() * 3);
    for tri in &topology.triangles {
        indices.push(tri[0]);
        indices.push(tri[1]);
        indices.push(tri[2]);
    }

    let (water_mesh, roof_mesh) = build_water_and_roof_meshes(map, face_topology, phase)?;

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

    Ok((terrain_mesh, water_mesh, roof_mesh))
}

pub(super) fn tile_color(
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
