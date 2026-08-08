// src/map/surface_topology/generator.rs
//! Derives Fixed24 triangulated `SurfaceTopology` from authoritative `HexFaceTopology`.

use super::twins::build_half_edge_twins;
use crate::map::face_topology::{HexFaceTopology, VertexId};
use crate::map::surface_topology::types::{
    SurfaceFace, SurfaceFaceId, SurfaceFaceSource, SurfaceHalfEdge, SurfaceHalfEdgeId,
    SurfaceTopology, SurfaceTopologyError, SurfaceTopologyStats, SurfaceVertex, SurfaceVertexId,
    SurfaceVertexSource,
};
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct SurfaceTopologyGeneratorPlugin;

impl Plugin for SurfaceTopologyGeneratorPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Computes twice the signed area of a 2D triangle.
#[inline]
fn triangle_signed_area(p0: Vec2, p1: Vec2, p2: Vec2) -> f32 {
    0.5 * ((p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y))
}

/// Derives deterministic `SurfaceTopology` from authoritative `HexFaceTopology`.
///
/// # Errors
/// Returns `SurfaceTopologyError` if a tile face is missing, vertex indices are invalid/non-finite,
/// derived triangles are degenerate, or half-edge topology is non-manifold.
#[allow(
    clippy::similar_names,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]
pub fn generate_surface_topology(
    face_topology: &HexFaceTopology,
) -> Result<SurfaceTopology, SurfaceTopologyError> {
    let mut surface = SurfaceTopology::default();

    let mut sorted_coords: Vec<HexCoord> = face_topology.hex_to_face.keys().copied().collect();
    sorted_coords.sort_by_key(|c| (c.q, c.r));

    let mut corner_map: Vec<Option<SurfaceVertexId>> = vec![None; face_topology.vertices.len()];
    let mut edge_map: HashMap<(usize, usize), SurfaceVertexId> = HashMap::new();

    for &coord in &sorted_coords {
        let &face_id = face_topology
            .hex_to_face
            .get(&coord)
            .ok_or(SurfaceTopologyError::MissingFaceForTile(coord))?;
        let face = face_topology.faces.get(face_id.index()).ok_or(
            SurfaceTopologyError::InvalidSourceFace {
                hex: coord,
                face: face_id,
            },
        )?;
        if face.hex != coord {
            return Err(SurfaceTopologyError::FaceHexMismatch {
                expected: coord,
                actual: face.hex,
            });
        }

        let mut corner_pos = [Vec2::ZERO; 6];
        let mut corner_vertex_ids = [VertexId::new(0); 6];
        let mut corner_surface_ids = [SurfaceVertexId::new(0); 6];

        for i in 0..6 {
            let v_id = face.vertices[i];
            let v_idx = v_id.index();
            if v_idx >= face_topology.vertices.len() {
                return Err(SurfaceTopologyError::InvalidSourceVertex {
                    face: coord,
                    vertex: v_id,
                });
            }
            let pos = face_topology.vertices[v_idx].position;
            if !pos.x.is_finite() || !pos.y.is_finite() {
                return Err(SurfaceTopologyError::NonFiniteSourceVertex(v_id));
            }
            corner_pos[i] = pos;
            corner_vertex_ids[i] = v_id;

            if let Some(s_idx) = corner_map[v_idx] {
                corner_surface_ids[i] = s_idx;
            } else {
                let s_idx = SurfaceVertexId::new(surface.vertices.len());
                surface.vertices.push(SurfaceVertex {
                    position: pos,
                    source: SurfaceVertexSource::HexCorner {
                        source_vertex: v_id,
                    },
                });
                corner_map[v_idx] = Some(s_idx);
                corner_surface_ids[i] = s_idx;
            }
        }

        let center_pos = (corner_pos[0]
            + corner_pos[1]
            + corner_pos[2]
            + corner_pos[3]
            + corner_pos[4]
            + corner_pos[5])
            / 6.0;
        let center_surface_id = SurfaceVertexId::new(surface.vertices.len());
        surface.vertices.push(SurfaceVertex {
            position: center_pos,
            source: SurfaceVertexSource::HexCenter { hex: coord },
        });

        let mut radial_surface_ids = [SurfaceVertexId::new(0); 6];
        for (i, r_id) in radial_surface_ids.iter_mut().enumerate() {
            let r_pos = 0.5 * (center_pos + corner_pos[i]);
            let s_idx = SurfaceVertexId::new(surface.vertices.len());
            surface.vertices.push(SurfaceVertex {
                position: r_pos,
                source: SurfaceVertexSource::HexRadialMidpoint {
                    hex: coord,
                    source_corner: corner_vertex_ids[i],
                },
            });
            *r_id = s_idx;
        }

        let mut edge_surface_ids = [SurfaceVertexId::new(0); 6];
        for i in 0..6 {
            let next = (i + 1) % 6;
            let va_id = face.vertices[i];
            let vb_id = face.vertices[next];
            let va_idx = va_id.index();
            let vb_idx = vb_id.index();

            let (source_a, source_b) = if va_idx < vb_idx {
                (va_id, vb_id)
            } else {
                (vb_id, va_id)
            };
            let key = (source_a.index(), source_b.index());

            if let Some(&e_idx) = edge_map.get(&key) {
                edge_surface_ids[i] = e_idx;
            } else {
                let e_pos = 0.5 * (corner_pos[i] + corner_pos[next]);
                let e_idx = SurfaceVertexId::new(surface.vertices.len());
                surface.vertices.push(SurfaceVertex {
                    position: e_pos,
                    source: SurfaceVertexSource::HexEdgeMidpoint { source_a, source_b },
                });
                edge_map.insert(key, e_idx);
                edge_surface_ids[i] = e_idx;
            }
        }

        let mut cell_faces = Vec::with_capacity(24);

        for i in 0..6 {
            let next = (i + 1) % 6;
            let ra = radial_surface_ids[i];
            let rb = radial_surface_ids[next];
            let va = corner_surface_ids[i];
            let vb = corner_surface_ids[next];
            let ea = edge_surface_ids[i];

            let tris = [
                [center_surface_id, ra, rb],
                [ra, va, ea],
                [ra, ea, rb],
                [rb, ea, vb],
            ];

            for (tri_idx, tri_verts) in tris.iter().enumerate() {
                #[allow(clippy::cast_possible_truncation)]
                let face_idx =
                    push_surface_triangle(&mut surface, *tri_verts, coord, i as u8, tri_idx as u8)?;
                cell_faces.push(face_idx);
            }
        }

        surface.hex_to_faces.insert(coord, cell_faces);
    }

    // Build half-edge reciprocal twins
    build_half_edge_twins(&mut surface)?;

    surface.stats = SurfaceTopologyStats {
        vertex_count: surface.vertices.len(),
        face_count: surface.faces.len(),
        half_edge_count: surface.half_edges.len(),
        paired_half_edge_count: surface
            .half_edges
            .iter()
            .filter(|h| h.twin.is_some())
            .count(),
        boundary_half_edge_count: surface
            .half_edges
            .iter()
            .filter(|h| h.twin.is_none())
            .count(),
    };

    Ok(surface)
}

fn push_surface_triangle(
    surface: &mut SurfaceTopology,
    tri_verts: [SurfaceVertexId; 3],
    hex: HexCoord,
    sector: u8,
    triangle: u8,
) -> Result<SurfaceFaceId, SurfaceTopologyError> {
    let p0 = surface.vertices[tri_verts[0].index()].position;
    let p1 = surface.vertices[tri_verts[1].index()].position;
    let p2 = surface.vertices[tri_verts[2].index()].position;
    let area = triangle_signed_area(p0, p1, p2);
    if !area.is_finite() || area <= 1e-6 {
        return Err(SurfaceTopologyError::DegenerateTriangle {
            hex,
            sector,
            triangle,
        });
    }

    let face_idx = SurfaceFaceId::new(surface.faces.len());
    let first_he_idx = SurfaceHalfEdgeId::new(surface.half_edges.len());

    surface.faces.push(SurfaceFace {
        vertices: tri_verts,
        boundary: first_he_idx,
        owner_hex: hex,
        source: SurfaceFaceSource { sector, triangle },
    });

    let h0 = first_he_idx;
    let h1 = SurfaceHalfEdgeId::new(first_he_idx.index() + 1);
    let h2 = SurfaceHalfEdgeId::new(first_he_idx.index() + 2);

    surface.half_edges.push(SurfaceHalfEdge {
        origin: tri_verts[0],
        destination: tri_verts[1],
        next: h1,
        prev: h2,
        twin: None,
        incident_face: face_idx,
    });
    surface.half_edges.push(SurfaceHalfEdge {
        origin: tri_verts[1],
        destination: tri_verts[2],
        next: h2,
        prev: h0,
        twin: None,
        incident_face: face_idx,
    });
    surface.half_edges.push(SurfaceHalfEdge {
        origin: tri_verts[2],
        destination: tri_verts[0],
        next: h0,
        prev: h1,
        twin: None,
        incident_face: face_idx,
    });

    Ok(face_idx)
}
