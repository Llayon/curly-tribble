// src/economy/mesh_gen/tests_cliff_gizmos.rs
//! Unit tests locking the cliff gizmo warped geometry derivation contract.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct CliffGizmosTestsPlugin;

impl Plugin for CliffGizmosTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economy::mesh_gen::cliff_gizmos::compute_cliff_gizmo_geometry;
    use crate::map::data::{
        CliffLowerSide, EdgeCoord, EdgeData, EdgeType, MapData, TileData, WorldSeed,
    };
    use crate::map::face_topology::edge_binding::bind_cliff_edges;
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::HexCoord;

    fn create_two_tile_map_with_side(lower: CliffLowerSide) -> MapData {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());
        let edge = EdgeCoord::new(c1, c2);
        map.edges.insert(
            edge,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: lower,
            },
        );
        map
    }

    #[test]
    fn exact_warped_segment_endpoints_match_topology_vertices() {
        let map = create_two_tile_map_with_side(CliffLowerSide::Unresolved);
        let seed = WorldSeed::new(42);
        let topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Topology generation failed");
        let bound = bind_cliff_edges(&map, &topology).expect("Edge binding failed");

        let geom = compute_cliff_gizmo_geometry(&bound.edges[0], &topology)
            .expect("Geometry computation failed");

        let he_a = &topology.half_edges[bound.edges[0].half_edge_a.index()];
        let expected_start = topology.vertices[he_a.origin.index()].position;
        let expected_end = topology.vertices[he_a.destination.index()].position;

        assert_eq!(geom.segment_start, expected_start);
        assert_eq!(geom.segment_end, expected_end);
    }

    #[test]
    fn arrow_targets_match_unresolved_a_and_b_lower_sides() {
        let seed = WorldSeed::new(42);

        for (lower, expected_count) in [
            (CliffLowerSide::Unresolved, 2),
            (CliffLowerSide::A, 1),
            (CliffLowerSide::B, 1),
        ] {
            let map = create_two_tile_map_with_side(lower);
            let topology =
                generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Organic)
                    .expect("Topology failed");
            let bound = bind_cliff_edges(&map, &topology).expect("Binding failed");
            let geom =
                compute_cliff_gizmo_geometry(&bound.edges[0], &topology).expect("Geometry failed");

            assert_eq!(geom.arrow_targets.len(), expected_count);

            let face_a_obj = &topology.faces[bound.edges[0].face_a.index()];
            let center_a: Vec2 = face_a_obj
                .vertices
                .iter()
                .map(|vid| topology.vertices[vid.index()].position)
                .sum::<Vec2>()
                / 6.0;

            let face_b_obj = &topology.faces[bound.edges[0].face_b.index()];
            let center_b: Vec2 = face_b_obj
                .vertices
                .iter()
                .map(|vid| topology.vertices[vid.index()].position)
                .sum::<Vec2>()
                / 6.0;

            match lower {
                CliffLowerSide::Unresolved => {
                    assert_eq!(geom.arrow_targets[0], center_a);
                    assert_eq!(geom.arrow_targets[1], center_b);
                }
                CliffLowerSide::A => {
                    assert_eq!(geom.arrow_targets[0], center_a);
                }
                CliffLowerSide::B => {
                    assert_eq!(geom.arrow_targets[0], center_b);
                }
            }
        }
    }
}
