// src/map/surface_topology/terrain_adapter.rs
//! Pure projection adapter from `SurfaceTopology` to `TerrainTopology` rendering representation.

use crate::map::surface_topology::types::{SurfaceTopology, SurfaceVertexId};
use crate::map::topology::TerrainTopology;
use crate::map::HexCoord;
use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTerrainAdapterPlugin;

impl Plugin for SurfaceTerrainAdapterPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceTerrainAdapterError {
    InvalidSurfaceVertex(SurfaceVertexId),
    VertexIndexOverflow { vertex: SurfaceVertexId },
    UnreferencedSurfaceVertex(SurfaceVertexId),
}

/// Projects a derived `SurfaceTopology` into a `TerrainTopology` rendering representation.
///
/// # Errors
/// Returns `SurfaceTerrainAdapterError` if surface contains out-of-bounds/overflowing vertex indices.
pub fn derive_terrain_topology_from_surface(
    surface: &SurfaceTopology,
) -> Result<TerrainTopology, SurfaceTerrainAdapterError> {
    if surface.vertices.is_empty() || surface.faces.is_empty() {
        return Ok(TerrainTopology::default());
    }

    let vertices_xz: Vec<Vec2> = surface.vertices.iter().map(|v| v.position).collect();

    let mut triangles = Vec::with_capacity(surface.faces.len());
    let mut triangle_cells = Vec::with_capacity(surface.faces.len());

    for face in &surface.faces {
        let mut tri = [0u32; 3];
        for (i, &v_id) in face.vertices.iter().enumerate() {
            if v_id.index() >= surface.vertices.len() {
                return Err(SurfaceTerrainAdapterError::InvalidSurfaceVertex(v_id));
            }
            tri[i] = u32::try_from(v_id.index())
                .map_err(|_| SurfaceTerrainAdapterError::VertexIndexOverflow { vertex: v_id })?;
        }
        triangles.push(tri);
        triangle_cells.push(face.owner_hex);
    }

    let mut vertex_influences = vec![Vec::<HexCoord>::new(); surface.vertices.len()];
    for face in &surface.faces {
        for &v_id in &face.vertices {
            vertex_influences[v_id.index()].push(face.owner_hex);
        }
    }

    for (idx, influence) in vertex_influences.iter_mut().enumerate() {
        if influence.is_empty() {
            return Err(SurfaceTerrainAdapterError::UnreferencedSurfaceVertex(
                SurfaceVertexId::new(idx),
            ));
        }
        influence.sort_by_key(|c| (c.q, c.r));
        influence.dedup();
    }

    Ok(TerrainTopology {
        vertices_xz,
        triangles,
        triangle_cells,
        vertex_influences,
    })
}
