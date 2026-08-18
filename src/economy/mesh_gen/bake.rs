// src/economy/mesh_gen/bake.rs
//! M5.1 bake-driven ground mesh generation (`SurfaceTerrainBake` pipeline).
//! One ground render vertex per `HeightNodeId`; cliff walls are appended as
//! duplicated vertices with indices `>= ground_vertex_count` so ground normals
//! never bleed into wall geometry.

use super::generator::{tile_color, OverlayGeometryError};
use super::overlay::build_water_and_roof_meshes;
use crate::game_state::{EditorPhase, FactionManager};
use crate::map::data::OceanState;
use crate::map::surface_gameplay::types::SurfaceGameplayMap;
use crate::map::terrain_bake::types::SurfaceTerrainBake;
use crate::map::terrain_gen::TerrainConfig;
use crate::map::{MapData, MAX_HEIGHT};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

/// Minimum squared 3D triangle area used to reject degenerate cliff wall triangles.
/// A vertical wall has zero XZ area by construction, so the check MUST be 3D.
const WALL_AREA_EPSILON_SQ: f32 = 1e-10;

pub struct TerrainBakeMeshPlugin;

impl Plugin for TerrainBakeMeshPlugin {
    fn build(&self, _app: &mut App) {}
}

/// World-space position of one baked ground vertex.
fn bake_world_pos(
    v: &crate::map::terrain_bake::types::TerrainBakeVertex,
    flat_surface: bool,
) -> [f32; 3] {
    let y = if flat_surface {
        0.0
    } else {
        v.normalized_height * MAX_HEIGHT
    };
    [v.position_xz.x, y, v.position_xz.y]
}

/// Color for a baked ground vertex. Fail-closed: empty `owner_hexes` is a
/// malformed bake and must never silently fall back to a default tile.
/// Buildability is policy-external: the lookup happens lazily only when the
/// build-area layer is visible (Sediments phase), otherwise `buildable` is
/// simply never consulted.
fn bake_vertex_color(
    bake_v: &crate::map::terrain_bake::types::TerrainBakeVertex,
    map: &MapData,
    phase: EditorPhase,
    faction_manager: &FactionManager,
    config: &TerrainConfig,
    faction_filter: bool,
    gameplay: &SurfaceGameplayMap,
) -> Result<[f32; 4], OverlayGeometryError> {
    if bake_v.owner_hexes.is_empty() {
        return Err(OverlayGeometryError::MissingBakeVertexOwner(
            bake_v.height_node,
        ));
    }
    let land_coord = bake_v.owner_hexes.iter().copied().find(|c| {
        map.get_tile(c.q, c.r)
            .is_some_and(|t| t.ocean_state == OceanState::Land)
    });
    let eval_coord = land_coord.unwrap_or_else(|| bake_v.owner_hexes[0]);
    let tile_data = map
        .get_tile(eval_coord.q, eval_coord.r)
        .copied()
        .unwrap_or_default();
    let buildable = if phase == EditorPhase::Sediments
        && config.build_area_layer.is_visible()
        && tile_data.ocean_state == OceanState::Land
    {
        gameplay
            .cells
            .get(&eval_coord)
            .ok_or(OverlayGeometryError::MissingGameplayCell(eval_coord))?
            .buildable
    } else {
        false
    };
    Ok(tile_color(
        &tile_data,
        phase,
        faction_manager,
        config,
        faction_filter,
        buildable,
    ))
}

/// Squared 3D area of a triangle; zero for co-planar degenerate walls.
fn wall_triangle_area_sq(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> f32 {
    let ab = Vec3::new(b[0] - a[0], b[1] - a[1], b[2] - a[2]);
    let ac = Vec3::new(c[0] - a[0], c[1] - a[1], c[2] - a[2]);
    ab.cross(ac).length_squared()
}

/// Appends cliff wall triangles for all bake cliff segments.
fn append_cliff_walls(
    bake: &SurfaceTerrainBake,
    map: &MapData,
    phase: EditorPhase,
    faction_manager: &FactionManager,
    config: &TerrainConfig,
    faction_filter: bool,
    gameplay: &SurfaceGameplayMap,
    flat_surface: bool,
    vertices: &mut Vec<[f32; 3]>,
    colors: &mut Vec<[f32; 4]>,
) -> Result<Vec<u32>, OverlayGeometryError> {
    let mut wall_indices = Vec::new();
    for segment in &bake.cliff_walls {
        let (ep0, ep1) = (&segment.endpoints[0], &segment.endpoints[1]);

        let p0v = bake
            .vertices
            .get(ep0.primary_node.index())
            .ok_or(OverlayGeometryError::InvalidHeightNode(ep0.primary_node))?;
        let t0v = bake
            .vertices
            .get(ep0.twin_node.index())
            .ok_or(OverlayGeometryError::InvalidHeightNode(ep0.twin_node))?;
        let p1v = bake
            .vertices
            .get(ep1.primary_node.index())
            .ok_or(OverlayGeometryError::InvalidHeightNode(ep1.primary_node))?;
        let t1v = bake
            .vertices
            .get(ep1.twin_node.index())
            .ok_or(OverlayGeometryError::InvalidHeightNode(ep1.twin_node))?;

        let p0 = bake_world_pos(p0v, flat_surface);
        let p1 = bake_world_pos(p1v, flat_surface);
        let t0 = bake_world_pos(t0v, flat_surface);
        let t1 = bake_world_pos(t1v, flat_surface);

        let color_primary_0 = bake_vertex_color(
            p0v,
            map,
            phase,
            faction_manager,
            config,
            faction_filter,
            gameplay,
        )?;
        let color_primary_1 = bake_vertex_color(
            p1v,
            map,
            phase,
            faction_manager,
            config,
            faction_filter,
            gameplay,
        )?;
        let color_twin_0 = bake_vertex_color(
            t0v,
            map,
            phase,
            faction_manager,
            config,
            faction_filter,
            gameplay,
        )?;
        let color_twin_1 = bake_vertex_color(
            t1v,
            map,
            phase,
            faction_manager,
            config,
            faction_filter,
            gameplay,
        )?;

        // Tapered walls (one collapsed endpoint) produce exactly one live
        // triangle; fully collapsed or equal-height segments produce none.
        let candidates: [([[f32; 3]; 3], [[f32; 4]; 3]); 2] = [
            (
                [p0, p1, t1],
                [color_primary_0, color_primary_1, color_twin_1],
            ),
            ([p0, t1, t0], [color_primary_0, color_twin_1, color_twin_0]),
        ];
        for (tri, tri_colors) in candidates {
            if wall_triangle_area_sq(tri[0], tri[1], tri[2]) <= WALL_AREA_EPSILON_SQ {
                continue;
            }
            let wall_base = u32::try_from(vertices.len())
                .map_err(|_| OverlayGeometryError::HeightNodeIndexOverflow(ep0.primary_node))?;
            for (pos, col) in tri.iter().zip(tri_colors) {
                vertices.push(*pos);
                colors.push(col);
            }
            wall_indices.extend_from_slice(&[wall_base, wall_base + 1, wall_base + 2]);
        }
    }
    Ok(wall_indices)
}

/// Creates map ground, water, and roof meshes from the M5.1 `SurfaceTerrainBake`.
///
/// One ground render vertex per `HeightNodeId`; cliff walls are appended as
/// duplicated vertices with indices `>= ground_vertex_count` so ground normals
/// never bleed into wall geometry.
///
/// # Errors
/// Returns `OverlayGeometryError` on any malformed bake reference or missing tile geometry.
#[allow(clippy::too_many_arguments)]
pub fn create_global_map_meshes_from_bake(
    map: &MapData,
    bake: &SurfaceTerrainBake,
    face_topology: &crate::map::face_topology::types::HexFaceTopology,
    phase: EditorPhase,
    faction_manager: &FactionManager,
    config: &TerrainConfig,
    gameplay: &SurfaceGameplayMap,
) -> Result<(Mesh, Option<Mesh>, Option<Mesh>), OverlayGeometryError> {
    let flat_surface = phase < EditorPhase::Height3D;
    let faction_filter = phase == EditorPhase::Factions;

    let mut vertices = Vec::with_capacity(bake.vertices.len());
    let mut colors = Vec::with_capacity(bake.vertices.len());

    for bake_v in &bake.vertices {
        vertices.push(bake_world_pos(bake_v, flat_surface));
        colors.push(bake_vertex_color(
            bake_v,
            map,
            phase,
            faction_manager,
            config,
            faction_filter,
            gameplay,
        )?);
    }

    let ground_vertex_count = vertices.len();

    let mut indices = Vec::with_capacity(bake.faces.len() * 3);
    for face in &bake.faces {
        for &node_id in &face.nodes {
            let idx = u32::try_from(node_id.index())
                .map_err(|_| OverlayGeometryError::HeightNodeIndexOverflow(node_id))?;
            indices.push(idx);
        }
    }

    if !flat_surface {
        let wall_indices = append_cliff_walls(
            bake,
            map,
            phase,
            faction_manager,
            config,
            faction_filter,
            gameplay,
            flat_surface,
            &mut vertices,
            &mut colors,
        )?;
        debug_assert!(
            wall_indices
                .iter()
                .all(|&i| i as usize >= ground_vertex_count),
            "wall index isolation invariant violated"
        );
        indices.extend_from_slice(&wall_indices);
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
