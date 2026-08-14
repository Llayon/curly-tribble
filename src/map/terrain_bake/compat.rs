// src/map/terrain_bake/compat.rs
//! Compatibility projection: `SurfaceTerrainBake` → `TerrainTopology`.
//! Used only as a reference/diagnostic resource; NOT the source of Y in M5.1+.

use crate::map::height_graph::types::HeightNodeId;
use crate::map::surface_topology::types::SurfaceFaceId;
use crate::map::terrain_bake::types::{SurfaceTerrainBake, TerrainBakeCompatError};
use crate::map::topology::TerrainTopology;
use crate::map::HexCoord;
use bevy::prelude::*;

pub struct TerrainBakeCompatPlugin;

impl Plugin for TerrainBakeCompatPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Projects `SurfaceTerrainBake` into a `TerrainTopology` for compatibility.
///
/// - `vertices_xz[i]` = `bake.vertices[i].position_xz`  (XZ per `HeightNodeId`)
/// - `triangles[i]`   = ground face node indices (wall triangles excluded)
/// - `vertex_influences[i]` = `bake.vertices[i].owner_hexes`
///
/// # Errors
/// Returns `TerrainBakeCompatError` on u32 index overflow.
pub fn derive_terrain_topology_from_bake(
    bake: &SurfaceTerrainBake,
) -> Result<TerrainTopology, TerrainBakeCompatError> {
    if bake.vertices.is_empty() && bake.faces.is_empty() {
        return Ok(TerrainTopology::default());
    }

    let vertices_xz = bake.vertices.iter().map(|v| v.position_xz).collect();

    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(bake.faces.len());
    let mut triangle_cells: Vec<HexCoord> = Vec::with_capacity(bake.faces.len());

    for bake_face in &bake.faces {
        let face_id = bake_face.surface_face;
        let mut tri = [0u32; 3];
        for (i, &node_id) in bake_face.nodes.iter().enumerate() {
            tri[i] = u32::try_from(node_id.index())
                .map_err(|_| TerrainBakeCompatError::VertexIndexOverflow(node_id))?;
            // Bounds: each node index should be < bake.vertices.len()
            if node_id.index() >= bake.vertices.len() {
                return Err(TerrainBakeCompatError::FaceNodeIndexOverflow {
                    face: face_id,
                    node: node_id,
                });
            }
        }
        triangles.push(tri);
        triangle_cells.push(bake_face.owner_hex);
    }

    let vertex_influences = bake
        .vertices
        .iter()
        .map(|v| v.owner_hexes.clone())
        .collect();

    Ok(TerrainTopology {
        vertices_xz,
        triangles,
        triangle_cells,
        vertex_influences,
    })
}

// ─── Unused type ID lint suppression ─────────────────────────────────────────

fn _assert_types() {
    let _: fn(HeightNodeId) = |_| {};
    let _: fn(SurfaceFaceId) = |_| {};
}
