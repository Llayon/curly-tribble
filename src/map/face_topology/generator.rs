/// Generator for deterministic hex face topology.
use crate::map::data::{MapData, HEX_SIZE};
use crate::map::face_topology::corner_key::{
    canonical_corner_key, corner_displacement, regular_corner_position,
};
use crate::map::face_topology::metrics::compute_topology_metrics;
use crate::map::face_topology::profiles::{profile_displacement, HexDeformationProfile};
use crate::map::face_topology::types::{
    FaceId, HalfEdge, HalfEdgeId, HexFace, HexFaceTopology, HexFaceTopologyError, MapVertex,
    SharedCornerKey, TopologyStats, VertexId,
};
use crate::map::face_topology::validation_complete::validate_complete_topology;
use crate::map::HexCoord;
use crate::map::WorldSeed;
use bevy::prelude::Vec2;
use std::collections::{HashMap, HashSet};

/// Generates deterministic `HexFaceTopology` from `MapData` and `WorldSeed`.
///
/// # Errors
/// Returns `HexFaceTopologyError` if the map is empty or if invalid geometry is produced.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn generate_hex_face_topology(
    map_data: &MapData,
    seed: WorldSeed,
) -> Result<HexFaceTopology, HexFaceTopologyError> {
    generate_hex_face_topology_with_profile(map_data, seed, HexDeformationProfile::Subtle)
}

/// Generates deterministic topology using one experimental diagnostic profile.
///
/// # Errors
/// Returns `HexFaceTopologyError` if the map or final profile geometry is invalid.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn generate_hex_face_topology_with_profile(
    map_data: &MapData,
    seed: WorldSeed,
    profile: HexDeformationProfile,
) -> Result<HexFaceTopology, HexFaceTopologyError> {
    if map_data.tiles.is_empty() {
        return Err(HexFaceTopologyError::EmptyMap);
    }

    let mut topology = HexFaceTopology::default();
    let mut key_to_vertex: HashMap<SharedCornerKey, VertexId> = HashMap::new();
    let mut corner_incident_faces: HashMap<SharedCornerKey, Vec<HexCoord>> = HashMap::new();

    let mut sorted_coords: Vec<HexCoord> = map_data.tiles.keys().copied().collect();
    sorted_coords.sort_by_key(|c| (c.q, c.r));

    // 1. Collect all unique corners and incident cells
    for &coord in &sorted_coords {
        for i in 0..6 {
            let key = canonical_corner_key(coord, i);
            corner_incident_faces.entry(key).or_default().push(coord);
        }
    }

    let mut sorted_keys: Vec<SharedCornerKey> = corner_incident_faces.keys().copied().collect();
    sorted_keys.sort_by_key(|k| {
        (
            k.first().q,
            k.first().r,
            k.second().q,
            k.second().r,
            k.third().q,
            k.third().r,
        )
    });

    let mut raw_displacements: HashMap<SharedCornerKey, Vec2> = HashMap::new();
    for &key in &sorted_keys {
        let disp = if profile == HexDeformationProfile::Subtle {
            corner_displacement(seed.value(), key, HEX_SIZE)
        } else {
            profile_displacement(seed.value(), key, HEX_SIZE, profile)
        };
        raw_displacements.insert(key, disp);
    }

    // Helper to evaluate 6 corner positions of a hex given current corner displacements
    let get_face_pts = |coord: HexCoord,
                        disp_map: &HashMap<SharedCornerKey, Vec2>|
     -> Result<[Vec2; 6], HexFaceTopologyError> {
        let mut pts = [Vec2::ZERO; 6];
        for i in 0..6 {
            let key = canonical_corner_key(coord, i);
            let base_pos = regular_corner_position(key)?;
            let Some(&disp) = disp_map.get(&key) else {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Missing displacement for corner {key:?}"
                )));
            };
            pts[i] = base_pos + disp;
        }
        Ok(pts)
    };

    let mut active_displacements = raw_displacements.clone();
    let mut stats = TopologyStats {
        profile,
        ..Default::default()
    };
    let mut reduced_keys = HashSet::new();

    let reduction_factors = [1.0f32, 0.875, 0.75, 0.625, 0.5, 0.375, 0.25, 0.125, 0.0];
    let mut reduction_index = 0;
    loop {
        let mut invalid_keys = HashSet::new();
        for (face_index, &coord) in sorted_coords.iter().enumerate() {
            let pts = get_face_pts(coord, &active_displacements)?;
            if crate::map::face_topology::validation::validate_face_geometry(
                &pts,
                FaceId::new(face_index),
            )
            .is_err()
            {
                for index in 0..6 {
                    invalid_keys.insert(canonical_corner_key(coord, index));
                }
            }
        }
        if invalid_keys.is_empty() {
            break;
        }
        let mut sorted_invalid_keys: Vec<_> = invalid_keys.into_iter().collect();
        sorted_invalid_keys.sort_unstable();
        if reduction_index + 1 >= reduction_factors.len() {
            for key in sorted_invalid_keys {
                active_displacements.insert(key, Vec2::ZERO);
                stats.regular_position_fallbacks += 1;
            }
            break;
        }
        reduction_index += 1;
        stats.reduction_rounds += 1;
        for key in sorted_invalid_keys {
            let Some(&raw_disp) = raw_displacements.get(&key) else {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Missing raw displacement for corner {key:?}"
                )));
            };
            active_displacements.insert(key, raw_disp * reduction_factors[reduction_index]);
            reduced_keys.insert(key);
            stats.reduced_displacement_fallbacks += 1;
        }
    }
    stats.reduced_vertices = reduced_keys.len();

    // 3. Construct MapVertex list
    for &key in &sorted_keys {
        let base_pos = regular_corner_position(key)?;
        let v_id = VertexId::new(topology.vertices.len());
        let Some(&disp) = active_displacements.get(&key) else {
            return Err(HexFaceTopologyError::ValidationFailed(format!(
                "Missing active displacement for corner {key:?}"
            )));
        };
        topology.vertices.push(MapVertex {
            position: base_pos + disp,
            canonical_key: key,
        });
        key_to_vertex.insert(key, v_id);
    }

    let mut directed_edge_map: HashMap<(VertexId, VertexId), HalfEdgeId> = HashMap::new();

    // 4. Construct faces and half-edges
    for (face_idx, &coord) in sorted_coords.iter().enumerate() {
        let f_id = FaceId::new(face_idx);
        let mut face_vertices = [VertexId::new(0); 6];
        for i in 0..6 {
            let key = canonical_corner_key(coord, i);
            let Some(&vertex_id) = key_to_vertex.get(&key) else {
                return Err(HexFaceTopologyError::CornerKeyMismatch(key));
            };
            face_vertices[i] = vertex_id;
        }

        let base_edge_idx = topology.half_edges.len();

        for i in 0..6 {
            let e_id = HalfEdgeId::new(base_edge_idx + i);
            let next_e_id = HalfEdgeId::new(base_edge_idx + (i + 1) % 6);
            let prev_e_id = HalfEdgeId::new(base_edge_idx + (i + 5) % 6);

            let origin = face_vertices[i];
            let destination = face_vertices[(i + 1) % 6];

            let edge_dir_key = (origin, destination);
            if directed_edge_map.insert(edge_dir_key, e_id).is_some() {
                return Err(HexFaceTopologyError::ValidationFailed(format!(
                    "Duplicate directed edge ({origin:?}, {destination:?})"
                )));
            }

            topology.half_edges.push(HalfEdge {
                origin,
                destination,
                next: next_e_id,
                prev: prev_e_id,
                twin: None,
                incident_face: f_id,
            });
        }

        let face = HexFace {
            hex: coord,
            boundary: HalfEdgeId::new(base_edge_idx),
            vertices: face_vertices,
        };

        topology.faces.push(face);
        topology.hex_to_face.insert(coord, f_id);
    }

    // 5. Connect Twin half-edges
    let edge_count = topology.half_edges.len();
    let mut paired_count = 0;
    let mut border_count = 0;

    for e_idx in 0..edge_count {
        let origin = topology.half_edges[e_idx].origin;
        let destination = topology.half_edges[e_idx].destination;

        let edge_rev_key = (destination, origin);
        if let Some(&twin_id) = directed_edge_map.get(&edge_rev_key) {
            topology.half_edges[e_idx].twin = Some(twin_id);
            paired_count += 1;
        } else {
            topology.half_edges[e_idx].twin = None;
            border_count += 1;
        }
    }

    // Half of paired count because each pair is counted twice in loop
    stats.paired_edge_count = paired_count / 2;
    stats.border_edge_count = border_count;
    stats.half_edge_count = edge_count;

    // 6. Compute diagnostic shape metrics.
    let metrics = compute_topology_metrics(&topology);
    stats.min_face_area = metrics.min_face_area;
    stats.max_face_area = metrics.max_face_area;
    stats.min_edge_length = metrics.min_edge_length;
    stats.max_edge_length = metrics.max_edge_length;
    stats.min_interior_angle = metrics.min_interior_angle;
    stats.max_interior_angle = metrics.max_interior_angle;
    stats.min_aspect_quality = metrics.min_aspect_quality;
    stats.max_aspect_quality = metrics.max_aspect_quality;
    stats.max_displacement = metrics.max_displacement;
    stats.average_displacement = metrics.average_displacement;

    // 6b. Hard safety cap: measured final displacement after backoff must stay
    // within the profile's absolute displacement cap. This is a generation
    // failure, not a visual warning.
    let cap_radius = profile.config().absolute_displacement_cap_ratio() * HEX_SIZE;
    if metrics.max_displacement > cap_radius * (1.0 + 1e-3) {
        return Err(HexFaceTopologyError::ProfileDisplacementCapExceeded {
            profile,
            max_displacement: metrics.max_displacement,
            cap_radius,
        });
    }

    topology.stats = stats;

    // 7. Final complete topology validation
    validate_complete_topology(&topology, map_data)?;

    Ok(topology)
}
