// src/map/height_constraints/tests_determinism.rs
//! Determinism tests verifying insertion order independence for compiled height constraints.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightConstraintDeterminismTestsPlugin;

impl Plugin for HeightConstraintDeterminismTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::EdgeCoord;
    use crate::map::data::{
        CliffLowerSide, EdgeData, EdgeType, LandscapeFeature, MapData, TileData, WorldSeed,
    };
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compiler::compile_height_constraints;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::HexCoord;

    fn lcg_next(state: &mut u64) -> u64 {
        *state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        *state
    }

    #[test]
    fn normal_reverse_lcg_insertion_order_determinism() {
        let coords = vec![
            HexCoord::new(0, 0),
            HexCoord::new(1, 0),
            HexCoord::new(0, 1),
            HexCoord::new(1, 1),
        ];

        let edges = vec![
            EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(1, 0)),
            EdgeCoord::new(HexCoord::new(0, 0), HexCoord::new(0, 1)),
            EdgeCoord::new(HexCoord::new(1, 0), HexCoord::new(1, 1)),
        ];

        let mut map_normal = MapData::default();
        for &c in &coords {
            map_normal.tiles.insert(
                c,
                TileData {
                    landscape_feature: LandscapeFeature::Mountain,
                    ..Default::default()
                },
            );
        }
        for &e in &edges {
            map_normal.edges.insert(
                e,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: CliffLowerSide::Unresolved,
                },
            );
        }

        let mut map_reverse = MapData::default();
        for &c in coords.iter().rev() {
            map_reverse.tiles.insert(
                c,
                TileData {
                    landscape_feature: LandscapeFeature::Mountain,
                    ..Default::default()
                },
            );
        }
        for &e in edges.iter().rev() {
            map_reverse.edges.insert(
                e,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: CliffLowerSide::Unresolved,
                },
            );
        }

        let mut map_lcg = MapData::default();
        let mut shuffled_coords = coords.clone();
        let mut lcg_state = 12345u64;
        shuffled_coords.sort_by_cached_key(|_| lcg_next(&mut lcg_state));
        for &c in &shuffled_coords {
            map_lcg.tiles.insert(
                c,
                TileData {
                    landscape_feature: LandscapeFeature::Mountain,
                    ..Default::default()
                },
            );
        }

        let mut shuffled_edges = edges.clone();
        shuffled_edges.sort_by_cached_key(|_| lcg_next(&mut lcg_state));
        for &e in &shuffled_edges {
            map_lcg.edges.insert(
                e,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: CliffLowerSide::Unresolved,
                },
            );
        }

        let seed = WorldSeed::new(42);
        let face_topology = generate_hex_face_topology_with_profile(
            &map_normal,
            seed,
            HexDeformationProfile::Subtle,
        )
        .expect("Face topology failed");
        let surface = generate_surface_topology(&face_topology).expect("Surface topology failed");

        let res_normal = compile_height_constraints(&map_normal, &surface).unwrap();
        let res_reverse = compile_height_constraints(&map_reverse, &surface).unwrap();
        let res_lcg = compile_height_constraints(&map_lcg, &surface).unwrap();

        assert_eq!(res_normal, res_reverse);
        assert_eq!(res_normal, res_lcg);
    }
}
