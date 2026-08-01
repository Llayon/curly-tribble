/// Generator for deterministic hex face topology.
use crate::map::data::{MapData, HEX_SIZE};
use crate::map::face_topology::corner_key::{
    canonical_corner_key, regular_corner_position, seed_for_corner,
};
use crate::map::face_topology::types::{
    FaceId, HalfEdge, HalfEdgeId, HexFace, HexFaceTopology, HexFaceTopologyError, MapVertex,
    SharedCornerKey, TopologyStats, VertexId,
};
use crate::map::face_topology::validation::{
    min_edge_length, signed_area, validate_complete_topology,
};
use crate::map::HexCoord;
use crate::map::WorldSeed;
use bevy::prelude::*;
use rand::prelude::*;
use std::collections::HashMap;

pub struct GeneratorPlugin;
impl Plugin for GeneratorPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Generates deterministic `HexFaceTopology` from `MapData` and `WorldSeed`.
///
/// # Errors
/// Returns `HexFaceTopologyError` if the map is empty or if invalid geometry is produced.
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn generate_hex_face_topology(
    map_data: &MapData,
    seed: WorldSeed,
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
    let max_disp_cap = 0.16 * HEX_SIZE;

    for &key in &sorted_keys {
        let corner_seed = seed_for_corner(seed.value(), key);
        let mut rng = rand::rngs::StdRng::seed_from_u64(corner_seed);
        let angle = rng.gen_range(0.0..std::f32::consts::TAU);
        let mag = rng
            .gen_range(0.08 * HEX_SIZE..0.12 * HEX_SIZE)
            .min(max_disp_cap);
        let disp = Vec2::new(mag * angle.cos(), mag * angle.sin());
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
            let disp = disp_map.get(&key).copied().unwrap_or(Vec2::ZERO);
            pts[i] = base_pos + disp;
        }
        Ok(pts)
    };

    let mut active_displacements = raw_displacements.clone();
    let mut stats = TopologyStats::default();

    // 2. Validate geometry per corner and apply displacement reduction fallback
    for &key in &sorted_keys {
        let raw_disp = raw_displacements.get(&key).copied().unwrap_or(Vec2::ZERO);
        let incident_coords = &corner_incident_faces[&key];

        let reduction_factors = [1.0f32, 0.75, 0.5, 0.25, 0.0];
        let mut chosen_factor = 0.0f32;
        let mut success = false;

        for &factor in &reduction_factors {
            active_displacements.insert(key, raw_disp * factor);

            let mut valid_all = true;
            for &coord in incident_coords {
                let pts = get_face_pts(coord, &active_displacements)?;
                if crate::map::face_topology::validation::validate_face_geometry(
                    &pts,
                    FaceId::new(0),
                )
                .is_err()
                {
                    valid_all = false;
                    break;
                }
            }

            if valid_all {
                chosen_factor = factor;
                success = true;
                break;
            }
        }

        if !success {
            active_displacements.insert(key, Vec2::ZERO);
            chosen_factor = 0.0;
        }

        if (chosen_factor - 1.0).abs() > 1e-4 {
            if chosen_factor > 0.0 {
                stats.reduced_displacement_fallbacks += 1;
            } else {
                stats.regular_position_fallbacks += 1;
            }
        }
    }

    // 3. Construct MapVertex list
    for &key in &sorted_keys {
        let base_pos = regular_corner_position(key)?;
        let disp = active_displacements
            .get(&key)
            .copied()
            .unwrap_or(Vec2::ZERO);
        let v_id = VertexId::new(topology.vertices.len());
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
            face_vertices[i] = key_to_vertex[&key];
        }

        let base_edge_idx = topology.half_edges.len();

        for i in 0..6 {
            let e_id = HalfEdgeId::new(base_edge_idx + i);
            let next_e_id = HalfEdgeId::new(base_edge_idx + (i + 1) % 6);
            let prev_e_id = HalfEdgeId::new(base_edge_idx + (i + 5) % 6);

            let origin = face_vertices[i];
            let destination = face_vertices[(i + 1) % 6];

            let edge_dir_key = (origin, destination);
            directed_edge_map.insert(edge_dir_key, e_id);

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

    // 6. Compute face areas and edge length bounds
    let mut min_area = f32::INFINITY;
    let mut max_area = f32::NEG_INFINITY;
    let mut min_edge = f32::INFINITY;

    for face in &topology.faces {
        let mut pts = [Vec2::ZERO; 6];
        for i in 0..6 {
            pts[i] = topology.vertices[face.vertices[i].index()].position;
        }
        let area = signed_area(&pts);
        if area < min_area {
            min_area = area;
        }
        if area > max_area {
            max_area = area;
        }
        let edge_len = min_edge_length(&pts);
        if edge_len < min_edge {
            min_edge = edge_len;
        }
    }

    stats.min_face_area = min_area;
    stats.max_face_area = max_area;
    stats.min_edge_length = min_edge;

    topology.stats = stats;

    // 7. Final complete topology validation
    validate_complete_topology(&topology, map_data)?;

    Ok(topology)
}
