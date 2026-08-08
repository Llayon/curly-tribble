// src/map/height_constraints/compiler.rs
//! Pure, total compilation of persistent landscape intent into semantic height constraints.

use crate::map::data::EdgeCoord;
use crate::map::data::{EdgeType, LandscapeFeature, MapData};
use crate::map::height_constraints::types::{
    CliffHeightConstraint, HeightConstraintCompileError, HeightConstraintSet,
    HeightConstraintStats, RegionHeightConstraint, RegionHeightIntent, SurfaceBoundarySegment,
};
use crate::map::surface_topology::types::{SurfaceHalfEdgeId, SurfaceTopology};
use bevy::prelude::*;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct HeightConstraintCompilerPlugin;

impl Plugin for HeightConstraintCompilerPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Compiles persistent authoring intent (`MapData` features and cliff edges) onto `SurfaceTopology` face and boundary identities.
///
/// # Errors
/// Returns `HeightConstraintCompileError` if mapped regions or cliff boundaries contain missing, invalid, or mismatched surface elements.
#[allow(clippy::too_many_lines)]
pub fn compile_height_constraints(
    map_data: &MapData,
    surface: &SurfaceTopology,
) -> Result<HeightConstraintSet, HeightConstraintCompileError> {
    if surface.vertices.is_empty() || surface.faces.is_empty() {
        return Ok(HeightConstraintSet::default());
    }

    let mut sorted_coords: Vec<_> = map_data.tiles.keys().copied().collect();
    sorted_coords.sort_by_key(|c| (c.q, c.r));

    let mut regions = Vec::new();

    for hex in sorted_coords {
        let tile = &map_data.tiles[&hex];
        let intent = match tile.landscape_feature {
            LandscapeFeature::Mountain => RegionHeightIntent::Mountain,
            LandscapeFeature::Plateau => RegionHeightIntent::Plateau,
            LandscapeFeature::Lake => RegionHeightIntent::Lake,
            LandscapeFeature::River => RegionHeightIntent::River,
            LandscapeFeature::None => continue,
        };

        let face_ids = surface
            .hex_to_faces
            .get(&hex)
            .ok_or(HeightConstraintCompileError::MissingSurfaceRegion(hex))?;

        let mut sorted_faces = Vec::with_capacity(face_ids.len());
        for &f_id in face_ids {
            let face = surface
                .faces
                .get(f_id.index())
                .ok_or(HeightConstraintCompileError::InvalidSurfaceFace(f_id))?;
            if face.owner_hex != hex {
                return Err(HeightConstraintCompileError::RegionOwnerMismatch {
                    hex,
                    face: f_id,
                    actual: face.owner_hex,
                });
            }
            sorted_faces.push(f_id);
        }
        sorted_faces.sort_by_key(|f| f.index());

        regions.push(RegionHeightConstraint {
            hex,
            intent,
            faces: sorted_faces,
        });
    }

    let mut boundary_index: HashMap<EdgeCoord, Vec<SurfaceBoundarySegment>> = HashMap::new();

    for (he_idx, he) in surface.half_edges.iter().enumerate() {
        let he_id = SurfaceHalfEdgeId::new(he_idx);
        if let Some(twin_id) = he.twin {
            if he_idx >= twin_id.index() {
                continue;
            }

            let face_a = surface.faces.get(he.incident_face.index()).ok_or(
                HeightConstraintCompileError::InvalidSurfaceFace(he.incident_face),
            )?;

            let twin = surface
                .half_edges
                .get(twin_id.index())
                .ok_or(HeightConstraintCompileError::MissingTwin(he_id))?;

            if twin.twin != Some(he_id) {
                return Err(HeightConstraintCompileError::NonReciprocalTwin {
                    a: he_id,
                    b: twin_id,
                });
            }

            let face_b = surface.faces.get(twin.incident_face.index()).ok_or(
                HeightConstraintCompileError::InvalidSurfaceFace(twin.incident_face),
            )?;

            if face_a.owner_hex != face_b.owner_hex {
                let logical_edge = EdgeCoord::new(face_a.owner_hex, face_b.owner_hex);

                let (half_edge_a, half_edge_b) = if face_a.owner_hex == logical_edge.a {
                    (he_id, twin_id)
                } else {
                    (twin_id, he_id)
                };

                boundary_index
                    .entry(logical_edge)
                    .or_default()
                    .push(SurfaceBoundarySegment {
                        half_edge_a,
                        half_edge_b,
                    });
            }
        }
    }

    for segments in boundary_index.values_mut() {
        segments.sort_by_key(|s| (s.half_edge_a.index(), s.half_edge_b.index()));
        segments.dedup();
    }

    let mut sorted_edges: Vec<_> = map_data
        .edges
        .iter()
        .filter(|(_, data)| data.edge_type == EdgeType::Cliff)
        .map(|(&edge, data)| (edge, data))
        .collect();
    sorted_edges.sort_by_key(|(edge, _)| (edge.a.q, edge.a.r, edge.b.q, edge.b.r));

    let mut cliffs = Vec::with_capacity(sorted_edges.len());

    for (logical_edge, edge_data) in sorted_edges {
        let segments = boundary_index
            .get(&logical_edge)
            .ok_or(HeightConstraintCompileError::MissingSurfaceBoundary(
                logical_edge,
            ))?
            .clone();

        if segments.is_empty() {
            return Err(HeightConstraintCompileError::IncompleteCliffSegments {
                edge: logical_edge,
            });
        }

        cliffs.push(CliffHeightConstraint {
            logical_edge,
            lower_side: edge_data.cliff_lower_side,
            segments,
        });
    }

    let referenced_surface_faces = regions.iter().map(|r| r.faces.len()).sum();
    let referenced_boundary_segments = cliffs.iter().map(|c| c.segments.len()).sum();

    let stats = HeightConstraintStats {
        region_count: regions.len(),
        cliff_count: cliffs.len(),
        referenced_surface_faces,
        referenced_boundary_segments,
    };

    Ok(HeightConstraintSet {
        regions,
        cliffs,
        stats,
    })
}
