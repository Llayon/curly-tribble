// src/economy/mesh_gen/overlay.rs
//! Water and roof overlay meshes built from `HexFaceTopology` XZ geometry.
//! Shared by the legacy topology generator and the M5.1 bake generator so the
//! overlay geometry is provably identical across both render paths.

use super::generator::OverlayGeometryError;
use crate::game_state::EditorPhase;
use crate::map::data::{OceanState, RoofState};
use crate::map::{LandscapeFeature, MapData};
use bevy::asset::RenderAssetUsages;
use bevy::mesh::Indices;
use bevy::prelude::*;
use bevy::render::render_resource::PrimitiveTopology;

pub struct OverlayMeshPlugin;

impl Plugin for OverlayMeshPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Builds water and roof overlay meshes from `HexFaceTopology` XZ geometry.
///
/// Shared by both the legacy topology generator and the M5.1 bake generator so
/// that overlay geometry is provably identical across the two render paths.
///
/// # Errors
/// Returns `OverlayGeometryError` if any tile's face or vertex lookup fails.
pub(crate) fn build_water_and_roof_meshes(
    map: &MapData,
    face_topology: &crate::map::face_topology::types::HexFaceTopology,
    phase: EditorPhase,
) -> Result<(Option<Mesh>, Option<Mesh>), OverlayGeometryError> {
    let flat_surface = phase < EditorPhase::Height3D;

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
        let center_y = if flat_surface || tile_data.ocean_state == OceanState::Ocean {
            0.0
        } else {
            map.get_hex_height(coord.q, coord.r)
        };

        if (tile_data.landscape_feature == LandscapeFeature::River
            || tile_data.landscape_feature == LandscapeFeature::Lake)
            && tile_data.ocean_state == OceanState::Land
        {
            let corners = extract_warped_face_corners(coord, face_topology)?;
            append_overlay_face(
                &mut water_vertices,
                &mut water_indices,
                &corners,
                center_y,
                &mut water_vertex_count,
            );
        }

        if tile_data.roof_state == RoofState::Roofed {
            let roof_y = center_y + 2.5;
            let corners = extract_warped_face_corners(coord, face_topology)?;
            append_overlay_face(
                &mut roof_vertices,
                &mut roof_indices,
                &corners,
                roof_y,
                &mut roof_vertex_count,
            );
        }
    }

    let water_mesh = create_optional_mesh(water_vertices, water_indices);
    let roof_mesh = create_optional_mesh(roof_vertices, roof_indices);

    Ok((water_mesh, roof_mesh))
}

pub(crate) fn extract_warped_face_corners(
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
