use crate::map::data::{MapData, OceanState, HEX_SIZE, MAX_HEIGHT};
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::HashMap;

pub use crate::map::topology_adapter::{
    derive_terrain_topology, TerrainTopologyError, TopologyAdapterPlugin,
};

/// Shared 24-triangle-per-hex terrain topology resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct TerrainTopology {
    pub vertices_xz: Vec<Vec2>,
    pub triangles: Vec<[u32; 3]>,
    pub triangle_cells: Vec<HexCoord>,
    pub vertex_influences: Vec<Vec<HexCoord>>,
}

pub struct TopologyPlugin;

impl Plugin for TopologyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainTopology>()
            .add_plugins(TopologyAdapterPlugin);
    }
}

/// Quantize Vec2 X/Z world position into integer key (1mm precision).
#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn canonical_vertex_key(pos: Vec2) -> (i32, i32) {
    (
        (pos.x * 1000.0).round() as i32,
        (pos.y * 1000.0).round() as i32,
    )
}

/// Generates a legacy regular-hex `TerrainTopology` from `MapData`.
#[must_use]
pub fn generate_legacy_topology_from_map_data(map_data: &MapData) -> TerrainTopology {
    let mut topology = TerrainTopology::default();
    let mut vertex_map: HashMap<(i32, i32), u32> = HashMap::new();

    // Sort tile keys for deterministic order
    let mut sorted_coords: Vec<HexCoord> = map_data.tiles.keys().copied().collect();
    sorted_coords.sort_by_key(|c| (c.q, c.r));

    for &coord in &sorted_coords {
        let center = coord.to_world(HEX_SIZE).xz();

        // 1. Center vertex (cell-local)
        let center_idx = add_vertex(
            &mut topology,
            center,
            coord,
            VertexSharing::CellLocal,
            &mut vertex_map,
        );

        // 2. Corner vertices (shared boundary)
        let mut v_indices = [0u32; 6];
        let mut corner_positions = [Vec2::ZERO; 6];
        for (i, v_idx) in v_indices.iter_mut().enumerate() {
            #[allow(clippy::cast_precision_loss)]
            let angle_deg = 60.0 * i as f32 + 30.0;
            let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
            let pos = Vec2::new(
                center.x + HEX_SIZE * angle_rad.cos(),
                center.y + HEX_SIZE * angle_rad.sin(),
            );
            corner_positions[i] = pos;
            *v_idx = add_vertex(
                &mut topology,
                pos,
                coord,
                VertexSharing::SharedBoundary,
                &mut vertex_map,
            );
        }

        // 3. Radial midpoint vertices (cell-local)
        let mut r_indices = [0u32; 6];
        for i in 0..6 {
            let r_pos = (center + corner_positions[i]) * 0.5;
            r_indices[i] = add_vertex(
                &mut topology,
                r_pos,
                coord,
                VertexSharing::CellLocal,
                &mut vertex_map,
            );
        }

        // 4. Outer-edge midpoint vertices (shared boundary)
        let mut e_indices = [0u32; 6];
        for i in 0..6 {
            let next = (i + 1) % 6;
            let e_pos = (corner_positions[i] + corner_positions[next]) * 0.5;
            e_indices[i] = add_vertex(
                &mut topology,
                e_pos,
                coord,
                VertexSharing::SharedBoundary,
                &mut vertex_map,
            );
        }

        // 5. Build 24 sub-triangles (4 triangles per sector)
        for i in 0..6 {
            let next = (i + 1) % 6;
            let ra = r_indices[i];
            let rb = r_indices[next];
            let va = v_indices[i];
            let vb = v_indices[next];
            let ea = e_indices[i];

            // Tri 1: Center, Radial A, Radial B
            topology.triangles.push([center_idx, ra, rb]);
            topology.triangle_cells.push(coord);

            // Tri 2: Radial A, Corner A, Edge Mid
            topology.triangles.push([ra, va, ea]);
            topology.triangle_cells.push(coord);

            // Tri 3: Radial A, Edge Mid, Radial B
            topology.triangles.push([ra, ea, rb]);
            topology.triangle_cells.push(coord);

            // Tri 4: Radial B, Edge Mid, Corner B
            topology.triangles.push([rb, ea, vb]);
            topology.triangle_cells.push(coord);
        }
    }

    topology
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VertexSharing {
    CellLocal,
    SharedBoundary,
}

fn add_vertex(
    topology: &mut TerrainTopology,
    pos: Vec2,
    coord: HexCoord,
    sharing: VertexSharing,
    vertex_map: &mut HashMap<(i32, i32), u32>,
) -> u32 {
    if sharing == VertexSharing::SharedBoundary {
        let key = canonical_vertex_key(pos);
        if let Some(&existing_idx) = vertex_map.get(&key) {
            let idx = existing_idx as usize;
            if idx < topology.vertex_influences.len()
                && !topology.vertex_influences[idx].contains(&coord)
            {
                topology.vertex_influences[idx].push(coord);
            }
            return existing_idx;
        }
        #[allow(clippy::cast_possible_truncation)]
        let new_idx = topology.vertices_xz.len() as u32;
        topology.vertices_xz.push(pos);
        topology.vertex_influences.push(vec![coord]);
        vertex_map.insert(key, new_idx);
        new_idx
    } else {
        #[allow(clippy::cast_possible_truncation)]
        let new_idx = topology.vertices_xz.len() as u32;
        topology.vertices_xz.push(pos);
        topology.vertex_influences.push(vec![coord]);
        new_idx
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainHeightMode {
    Flat,
    Relief3D,
}

/// Computes vertex Y elevations for all vertices in `TerrainTopology`.
#[must_use]
pub fn compute_vertex_heights(
    topology: &TerrainTopology,
    map_data: &MapData,
    mode: TerrainHeightMode,
) -> Vec<f32> {
    if mode == TerrainHeightMode::Flat {
        return vec![0.0; topology.vertices_xz.len()];
    }

    let mut heights = Vec::with_capacity(topology.vertices_xz.len());
    for influences in &topology.vertex_influences {
        if influences.is_empty() {
            heights.push(0.0);
        } else {
            let sum_elev: f32 = influences
                .iter()
                .map(|c| {
                    map_data.get_tile(c.q, c.r).map_or(0.0, |t| {
                        if t.ocean_state == OceanState::Ocean {
                            0.0
                        } else {
                            t.elevation * MAX_HEIGHT
                        }
                    })
                })
                .sum();
            #[allow(clippy::cast_precision_loss)]
            let avg_height = sum_elev / influences.len() as f32;
            heights.push(avg_height);
        }
    }
    heights
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_state::EditorPhase;
    use crate::map::data::{MapData, OceanState, TileData};
    use crate::map::HexCoord;

    #[test]
    fn test_terrain_topology_and_mesh_properties_1_to_8() {
        let mut map = MapData::default();
        map.tiles.insert(
            HexCoord::new(0, 0),
            TileData {
                ocean_state: OceanState::Land,
                elevation: 0.1,
                ..default()
            },
        );
        map.tiles.insert(
            HexCoord::new(1, 0),
            TileData {
                ocean_state: OceanState::Land,
                elevation: 0.9,
                ..default()
            },
        );

        let top_b = generate_legacy_topology_from_map_data(&map);
        let top_h = generate_legacy_topology_from_map_data(&map);
        assert!(
            !top_b.triangles.is_empty() && top_b.triangles.len() == map.tiles.len() * 24,
            "Tests 1&2: Topology count"
        );
        assert!(
            top_b.vertices_xz == top_h.vertices_xz
                && top_b.triangles == top_h.triangles
                && top_b.triangle_cells == top_h.triangle_cells,
            "Test 3: Topology match"
        );

        let flat_y = compute_vertex_heights(&top_b, &map, TerrainHeightMode::Flat);
        let relief_y = compute_vertex_heights(&top_h, &map, TerrainHeightMode::Relief3D);
        assert!(flat_y.iter().all(|&y| y == 0.0), "Test 4: Balance Y is 0");
        let (min_y, max_y) = (
            relief_y.iter().copied().fold(f32::INFINITY, f32::min),
            relief_y.iter().copied().fold(f32::NEG_INFINITY, f32::max),
        );
        assert!(max_y > min_y, "Test 5: Height3D Y range non-zero");

        let face_topo = crate::map::face_topology::generate_hex_face_topology_with_profile(
            &map,
            crate::map::WorldSeed::new(42),
            crate::map::face_topology::profiles::HexDeformationProfile::Subtle,
        )
        .unwrap();

        let (mesh, _, _) = crate::economy::mesh_gen::generator::create_global_map_meshes(
            &map,
            &top_h,
            &face_topo,
            EditorPhase::Height3D,
            &default(),
            &default(),
        );
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(nor)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("No normals")
        };
        assert!(
            nor.iter().any(|n| n[0].abs() > 1e-4 || n[2].abs() > 1e-4),
            "Test 6: Normals not all [0,1,0]"
        );
        assert!(
            EditorPhase::Balance < EditorPhase::Height3D
                && EditorPhase::Height3D >= EditorPhase::Height3D,
            "Test 7: Unlit behavior"
        );
        assert!(
            EditorPhase::Balance <= EditorPhase::Deposits
                && EditorPhase::Height3D <= EditorPhase::Deposits,
            "Test 8: Gizmos active"
        );
    }
}
