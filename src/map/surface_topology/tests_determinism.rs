// src/map/surface_topology/tests_determinism.rs
//! Tile insertion order determinism tests.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct SurfaceTopologyDeterminismTestsPlugin;

impl Plugin for SurfaceTopologyDeterminismTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::HexCoord;

    #[test]
    fn normal_reverse_lcg_tile_insertion_order_determinism() {
        let mut map_normal = MapData::default();
        let mut map_reverse = MapData::default();
        let mut map_lcg = MapData::default();

        let coords: Vec<_> = (0..4)
            .flat_map(|q| (0..4).map(move |r| HexCoord::new(q, r)))
            .collect();

        for &c in &coords {
            map_normal.tiles.insert(c, TileData::default());
        }
        for &c in coords.iter().rev() {
            map_reverse.tiles.insert(c, TileData::default());
        }
        let mut lcg_coords = coords.clone();
        lcg_coords.sort_by_key(|c| {
            c.q.unsigned_abs()
                .wrapping_mul(1664525)
                .wrapping_add(c.r.unsigned_abs().wrapping_mul(1013904223))
                % 4294967291
        });
        for &c in &lcg_coords {
            map_lcg.tiles.insert(c, TileData::default());
        }

        let seed = WorldSeed::new(42);
        let top_normal = generate_hex_face_topology_with_profile(
            &map_normal,
            seed,
            HexDeformationProfile::Organic,
        )
        .expect("Topology failed");
        let top_reverse = generate_hex_face_topology_with_profile(
            &map_reverse,
            seed,
            HexDeformationProfile::Organic,
        )
        .expect("Topology failed");
        let top_lcg =
            generate_hex_face_topology_with_profile(&map_lcg, seed, HexDeformationProfile::Organic)
                .expect("Topology failed");

        let surface_normal = generate_surface_topology(&top_normal).expect("Surface failed");
        let surface_reverse = generate_surface_topology(&top_reverse).expect("Surface failed");
        let surface_lcg = generate_surface_topology(&top_lcg).expect("Surface failed");

        assert_eq!(surface_normal, surface_reverse);
        assert_eq!(surface_normal, surface_lcg);
    }
}
