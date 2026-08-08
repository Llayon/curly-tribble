// src/map/height_constraints/validation.rs
//! Pure, bidirectional completeness validation of `HeightConstraintSet` against `MapData` and `SurfaceTopology`.

use crate::map::data::EdgeCoord;
use crate::map::data::{EdgeType, LandscapeFeature, MapData};
use crate::map::height_constraints::types::{
    HeightConstraintCompileError, HeightConstraintSet, RegionHeightIntent, SurfaceBoundarySegment,
};
use crate::map::surface_topology::types::{SurfaceHalfEdgeId, SurfaceTopology};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[allow(dead_code)]
pub struct HeightConstraintValidationPlugin;

impl Plugin for HeightConstraintValidationPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Performs 2-way completeness validation proving `HeightConstraintSet` is the exact complete representation of persistent authoring intent on `SurfaceTopology`.
///
/// # Errors
/// Returns `HeightConstraintCompileError` if region or cliff constraints are incomplete, mismatched, unauthored, or topologically invalid.
#[allow(clippy::too_many_lines)]
pub fn validate_height_constraint_set(
    constraints: &HeightConstraintSet,
    map_data: &MapData,
    surface: &SurfaceTopology,
) -> Result<(), HeightConstraintCompileError> {
    if surface.vertices.is_empty() || surface.faces.is_empty() {
        if constraints.regions.is_empty() && constraints.cliffs.is_empty() {
            return Ok(());
        }
        return Err(HeightConstraintCompileError::UnauthoredRegionConstraint(
            crate::map::HexCoord::new(0, 0),
        ));
    }

    let mut expected_regions = HashMap::new();
    for (&hex, tile) in &map_data.tiles {
        let intent = match tile.landscape_feature {
            LandscapeFeature::Mountain => RegionHeightIntent::Mountain,
            LandscapeFeature::Plateau => RegionHeightIntent::Plateau,
            LandscapeFeature::Lake => RegionHeightIntent::Lake,
            LandscapeFeature::River => RegionHeightIntent::River,
            LandscapeFeature::None => continue,
        };
        expected_regions.insert(hex, intent);
    }

    if constraints.regions.len() != expected_regions.len() {
        return Err(HeightConstraintCompileError::UnauthoredRegionConstraint(
            crate::map::HexCoord::new(0, 0),
        ));
    }

    let mut seen_region_hexes = HashSet::with_capacity(constraints.regions.len());

    for region in &constraints.regions {
        if !seen_region_hexes.insert(region.hex) {
            return Err(HeightConstraintCompileError::UnauthoredRegionConstraint(
                region.hex,
            ));
        }

        let &expected_intent = expected_regions.get(&region.hex).ok_or(
            HeightConstraintCompileError::UnauthoredRegionConstraint(region.hex),
        )?;

        if region.intent != expected_intent {
            return Err(HeightConstraintCompileError::UnauthoredRegionConstraint(
                region.hex,
            ));
        }

        if region.faces.is_empty() {
            return Err(HeightConstraintCompileError::IncompleteRegionFaces { hex: region.hex });
        }

        let expected_faces = surface.hex_to_faces.get(&region.hex).ok_or(
            HeightConstraintCompileError::MissingSurfaceRegion(region.hex),
        )?;

        if region.faces.len() != expected_faces.len() {
            return Err(HeightConstraintCompileError::IncompleteRegionFaces { hex: region.hex });
        }

        for (i, &f_id) in region.faces.iter().enumerate() {
            let face = surface
                .faces
                .get(f_id.index())
                .ok_or(HeightConstraintCompileError::InvalidSurfaceFace(f_id))?;
            if face.owner_hex != region.hex {
                return Err(HeightConstraintCompileError::RegionOwnerMismatch {
                    hex: region.hex,
                    face: f_id,
                    actual: face.owner_hex,
                });
            }
            if i > 0 && region.faces[i - 1].index() >= f_id.index() {
                return Err(HeightConstraintCompileError::IncompleteRegionFaces {
                    hex: region.hex,
                });
            }
        }
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

    let mut expected_cliffs = HashMap::new();
    for (&edge, edge_data) in &map_data.edges {
        if edge_data.edge_type == EdgeType::Cliff {
            expected_cliffs.insert(edge, edge_data.cliff_lower_side);
        }
    }

    if constraints.cliffs.len() != expected_cliffs.len() {
        return Err(HeightConstraintCompileError::UnauthoredCliffConstraint(
            EdgeCoord::new(
                crate::map::HexCoord::new(0, 0),
                crate::map::HexCoord::new(1, 0),
            ),
        ));
    }

    let mut seen_cliff_edges = HashSet::with_capacity(constraints.cliffs.len());

    for cliff in &constraints.cliffs {
        if !seen_cliff_edges.insert(cliff.logical_edge) {
            return Err(HeightConstraintCompileError::UnauthoredCliffConstraint(
                cliff.logical_edge,
            ));
        }

        let &expected_lower_side = expected_cliffs.get(&cliff.logical_edge).ok_or(
            HeightConstraintCompileError::UnauthoredCliffConstraint(cliff.logical_edge),
        )?;

        if cliff.lower_side != expected_lower_side {
            return Err(HeightConstraintCompileError::UnauthoredCliffConstraint(
                cliff.logical_edge,
            ));
        }

        let expected_segments = boundary_index.get(&cliff.logical_edge).ok_or(
            HeightConstraintCompileError::MissingSurfaceBoundary(cliff.logical_edge),
        )?;

        if &cliff.segments != expected_segments {
            return Err(HeightConstraintCompileError::IncompleteCliffSegments {
                edge: cliff.logical_edge,
            });
        }

        for (i, segment) in cliff.segments.iter().enumerate() {
            let he_a = surface.half_edges.get(segment.half_edge_a.index()).ok_or(
                HeightConstraintCompileError::InvalidSurfaceHalfEdge(segment.half_edge_a),
            )?;
            let he_b = surface.half_edges.get(segment.half_edge_b.index()).ok_or(
                HeightConstraintCompileError::InvalidSurfaceHalfEdge(segment.half_edge_b),
            )?;

            if he_a.twin != Some(segment.half_edge_b) || he_b.twin != Some(segment.half_edge_a) {
                return Err(HeightConstraintCompileError::NonReciprocalTwin {
                    a: segment.half_edge_a,
                    b: segment.half_edge_b,
                });
            }

            let face_a = surface.faces.get(he_a.incident_face.index()).ok_or(
                HeightConstraintCompileError::InvalidSurfaceFace(he_a.incident_face),
            )?;
            let face_b = surface.faces.get(he_b.incident_face.index()).ok_or(
                HeightConstraintCompileError::InvalidSurfaceFace(he_b.incident_face),
            )?;

            if face_a.owner_hex != cliff.logical_edge.a {
                return Err(HeightConstraintCompileError::BoundaryOwnerMismatch {
                    edge: cliff.logical_edge,
                    half_edge: segment.half_edge_a,
                    expected: cliff.logical_edge.a,
                    actual: face_a.owner_hex,
                });
            }
            if face_b.owner_hex != cliff.logical_edge.b {
                return Err(HeightConstraintCompileError::BoundaryOwnerMismatch {
                    edge: cliff.logical_edge,
                    half_edge: segment.half_edge_b,
                    expected: cliff.logical_edge.b,
                    actual: face_b.owner_hex,
                });
            }

            if i > 0
                && (
                    cliff.segments[i - 1].half_edge_a.index(),
                    cliff.segments[i - 1].half_edge_b.index(),
                ) >= (segment.half_edge_a.index(), segment.half_edge_b.index())
            {
                return Err(HeightConstraintCompileError::IncompleteCliffSegments {
                    edge: cliff.logical_edge,
                });
            }
        }
    }

    let referenced_surface_faces: usize = constraints.regions.iter().map(|r| r.faces.len()).sum();
    let referenced_boundary_segments: usize =
        constraints.cliffs.iter().map(|c| c.segments.len()).sum();

    if constraints.stats.region_count != constraints.regions.len()
        || constraints.stats.cliff_count != constraints.cliffs.len()
        || constraints.stats.referenced_surface_faces != referenced_surface_faces
        || constraints.stats.referenced_boundary_segments != referenced_boundary_segments
    {
        return Err(HeightConstraintCompileError::UnauthoredRegionConstraint(
            crate::map::HexCoord::new(0, 0),
        ));
    }

    Ok(())
}
