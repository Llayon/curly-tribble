// src/map/tools/tests_cliff_edit/tests_matrix.rs
//! 144-case proof matrix and insertion-order determinism tests for warped edge picking.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct CliffEditTestsMatrixPlugin;

impl Plugin for CliffEditTestsMatrixPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::{MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::tools::cliff_picking::{classify_side, LogicalEdgeSide};
    use crate::map::tools::landscape_edge_picker::build_landscape_edge_pick_index;
    use crate::map::HexCoord;

    #[test]
    fn canonical_144_case_matrix_picker_index_audit() {
        let mut cases = 0;
        let mut total_interior_edges = 0;

        for (shape, map) in q::all_shapes() {
            for seed_val in q::FAST_SEEDS {
                for profile in q::all_profiles() {
                    cases += 1;
                    let seed = WorldSeed::new(seed_val);
                    let topology = generate_hex_face_topology_with_profile(&map, seed, profile)
                        .expect("Topology generation failed");

                    let pick_index = build_landscape_edge_pick_index(&topology)
                        .expect("Pick index build failed");

                    assert_eq!(
                        pick_index.edges.len(),
                        topology.stats.paired_edge_count,
                        "Shape {shape} seed {seed_val} profile {profile:?}: index length mismatch"
                    );

                    total_interior_edges += pick_index.edges.len();

                    if shape == "1x1" {
                        assert_eq!(
                            pick_index.edges.len(),
                            0,
                            "1x1 shape must have 0 interior editable edges"
                        );
                    }

                    for e in &pick_index.edges {
                        let face_a_obj = &topology.faces[e.face_a.index()];
                        let face_b_obj = &topology.faces[e.face_b.index()];

                        assert_eq!(face_a_obj.hex, e.logical_edge.a);
                        assert_eq!(face_b_obj.hex, e.logical_edge.b);

                        let he_a = &topology.half_edges[e.half_edge_a.index()];
                        let he_b = &topology.half_edges[e.half_edge_b.index()];

                        assert_eq!(he_a.incident_face, e.face_a);
                        assert_eq!(he_b.incident_face, e.face_b);
                        assert_eq!(he_a.twin, Some(e.half_edge_b));
                        assert_eq!(he_b.twin, Some(e.half_edge_a));
                        assert_eq!(he_a.origin, he_b.destination);
                        assert_eq!(he_a.destination, he_b.origin);

                        assert!(e.segment_start.is_finite());
                        assert!(e.segment_end.is_finite());
                        assert!(e.center_a.is_finite());
                        assert!(e.center_b.is_finite());

                        let sign_a = classify_side(
                            e.center_a,
                            e.segment_start,
                            e.segment_end,
                            e.center_a,
                            e.center_b,
                        );
                        let sign_b = classify_side(
                            e.center_b,
                            e.segment_start,
                            e.segment_end,
                            e.center_a,
                            e.center_b,
                        );
                        assert_eq!(sign_a, Some(LogicalEdgeSide::A));
                        assert_eq!(sign_b, Some(LogicalEdgeSide::B));
                    }
                }
            }
        }

        assert_eq!(cases, 144);
        assert!(total_interior_edges > 0);
    }

    #[test]
    fn determinism_normal_reverse_lcg_tile_and_edge_index() {
        let mut map_normal = MapData::default();
        let mut map_reverse = MapData::default();
        let mut map_lcg = MapData::default();

        let coords: Vec<_> = (0..5)
            .flat_map(|q| (0..5).map(move |r| HexCoord::new(q, r)))
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

        let idx_normal = build_landscape_edge_pick_index(&top_normal).expect("Pick index failed");
        let idx_reverse = build_landscape_edge_pick_index(&top_reverse).expect("Pick index failed");
        let idx_lcg = build_landscape_edge_pick_index(&top_lcg).expect("Pick index failed");

        assert_eq!(idx_normal, idx_reverse);
        assert_eq!(idx_normal, idx_lcg);
    }
}
