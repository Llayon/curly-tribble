//! Adapter converting authoritative `HexFaceTopology` into derived 24-triangle `TerrainTopology`.
use crate::map::data::MapData;
use crate::map::face_topology::{HexFaceTopology, VertexId};
use crate::map::topology::TerrainTopology;
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::HashMap;

pub struct TopologyAdapterPlugin;

impl Plugin for TopologyAdapterPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainTopologyError {
    MissingFaceForTile(HexCoord),
    InvalidSourceVertex {
        face: HexCoord,
        vertex: VertexId,
    },
    FaceHexMismatch {
        expected: HexCoord,
        actual: HexCoord,
    },
    NonFiniteSourceVertex(VertexId),
    DegenerateDerivedTriangle {
        hex: HexCoord,
        sector: u8,
        triangle: u8,
    },
}

/// Computes twice the signed area of a 2D triangle.
#[inline]
fn triangle_signed_area(p0: Vec2, p1: Vec2, p2: Vec2) -> f32 {
    0.5 * ((p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y))
}

/// Derives deterministic `TerrainTopology` from authoritative `HexFaceTopology`.
///
/// # Errors
/// Returns `TerrainTopologyError` if a tile face is missing, vertex indices are invalid/non-finite,
/// or derived triangles are degenerate.
#[allow(
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]
pub fn derive_terrain_topology(
    map_data: &MapData,
    face_topology: &HexFaceTopology,
) -> Result<TerrainTopology, TerrainTopologyError> {
    let mut topology = TerrainTopology::default();
    let mut sorted_coords: Vec<HexCoord> = map_data.tiles.keys().copied().collect();
    sorted_coords.sort_by_key(|c| (c.q, c.r));

    let mut corner_map: Vec<Option<u32>> = vec![None; face_topology.vertices.len()];
    let mut edge_map: HashMap<(usize, usize), u32> = HashMap::new();

    for &coord in &sorted_coords {
        let &face_id = face_topology
            .hex_to_face
            .get(&coord)
            .ok_or(TerrainTopologyError::MissingFaceForTile(coord))?;
        let face = &face_topology.faces[face_id.index()];
        if face.hex != coord {
            return Err(TerrainTopologyError::FaceHexMismatch {
                expected: coord,
                actual: face.hex,
            });
        }

        let mut corner_pos = [Vec2::ZERO; 6];
        let mut corner_indices = [0u32; 6];

        for i in 0..6 {
            let v_id = face.vertices[i];
            let v_idx = v_id.index();
            if v_idx >= face_topology.vertices.len() {
                return Err(TerrainTopologyError::InvalidSourceVertex {
                    face: coord,
                    vertex: v_id,
                });
            }
            let pos = face_topology.vertices[v_idx].position;
            if !pos.x.is_finite() || !pos.y.is_finite() {
                return Err(TerrainTopologyError::NonFiniteSourceVertex(v_id));
            }
            corner_pos[i] = pos;

            if let Some(d_idx) = corner_map[v_idx] {
                corner_indices[i] = d_idx;
                topology.vertex_influences[d_idx as usize].push(coord);
            } else {
                #[allow(clippy::cast_possible_truncation)]
                let d_idx = topology.vertices_xz.len() as u32;
                topology.vertices_xz.push(pos);
                topology.vertex_influences.push(vec![coord]);
                corner_map[v_idx] = Some(d_idx);
                corner_indices[i] = d_idx;
            }
        }

        let center_pos = (corner_pos[0]
            + corner_pos[1]
            + corner_pos[2]
            + corner_pos[3]
            + corner_pos[4]
            + corner_pos[5])
            / 6.0;
        #[allow(clippy::cast_possible_truncation)]
        let center_idx = topology.vertices_xz.len() as u32;
        topology.vertices_xz.push(center_pos);
        topology.vertex_influences.push(vec![coord]);

        let mut radial_indices = [0u32; 6];
        for (i, r_idx) in radial_indices.iter_mut().enumerate() {
            let r_pos = 0.5 * (center_pos + corner_pos[i]);
            #[allow(clippy::cast_possible_truncation)]
            let idx = topology.vertices_xz.len() as u32;
            topology.vertices_xz.push(r_pos);
            topology.vertex_influences.push(vec![coord]);
            *r_idx = idx;
        }

        let mut edge_indices = [0u32; 6];
        for i in 0..6 {
            let next = (i + 1) % 6;
            let va_idx = face.vertices[i].index();
            let vb_idx = face.vertices[next].index();
            let key = (va_idx.min(vb_idx), va_idx.max(vb_idx));

            if let Some(&e_idx) = edge_map.get(&key) {
                edge_indices[i] = e_idx;
                topology.vertex_influences[e_idx as usize].push(coord);
            } else {
                let e_pos = 0.5 * (corner_pos[i] + corner_pos[next]);
                #[allow(clippy::cast_possible_truncation)]
                let e_idx = topology.vertices_xz.len() as u32;
                topology.vertices_xz.push(e_pos);
                topology.vertex_influences.push(vec![coord]);
                edge_map.insert(key, e_idx);
                edge_indices[i] = e_idx;
            }
        }

        for i in 0..6 {
            let next = (i + 1) % 6;
            let ra = radial_indices[i];
            let rb = radial_indices[next];
            let va = corner_indices[i];
            let vb = corner_indices[next];
            let ea = edge_indices[i];

            let tris = [
                [center_idx, ra, rb],
                [ra, va, ea],
                [ra, ea, rb],
                [rb, ea, vb],
            ];

            for (tri_idx, tri) in tris.iter().enumerate() {
                let p0 = topology.vertices_xz[tri[0] as usize];
                let p1 = topology.vertices_xz[tri[1] as usize];
                let p2 = topology.vertices_xz[tri[2] as usize];
                let area = triangle_signed_area(p0, p1, p2);
                if !area.is_finite() || area.abs() <= 1e-6 {
                    return Err(TerrainTopologyError::DegenerateDerivedTriangle {
                        hex: coord,
                        sector: i as u8,
                        triangle: tri_idx as u8,
                    });
                }
                topology.triangles.push(*tri);
                topology.triangle_cells.push(coord);
            }
        }
    }

    for influences in &mut topology.vertex_influences {
        influences.sort_by_key(|c| (c.q, c.r));
        influences.dedup();
    }

    Ok(topology)
}
