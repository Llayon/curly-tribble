//! Integration tests for HexFaceTopology -> TerrainTopology adapter.
#[cfg(test)]
mod tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::corner_key::{canonical_corner_key, regular_corner_position};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::face_topology::FaceId;
    use crate::map::topology::{derive_terrain_topology, TerrainTopologyError};
    use crate::map::{HexCoord, WorldSeed};
    use std::collections::HashMap;

    fn canonical_40x40_map() -> MapData {
        let mut map = MapData::default();
        for r in 0..40i32 {
            let offset = r >> 1;
            for q in -offset..(40 - offset) {
                map.tiles.insert(HexCoord::new(q, r), TileData::default());
            }
        }
        map.width = 40;
        map.height = 40;
        map
    }

    fn lcg_rand(state: &mut u64) -> u64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
        *state
    }

    #[test]
    fn derived_shared_corners_match_source_map_vertices_bit_identically() {
        let map = canonical_40x40_map();
        let face_topo = generate_hex_face_topology_with_profile(
            &map,
            WorldSeed::new(42),
            HexDeformationProfile::Subtle,
        )
        .expect("generate face topology");

        let derived = derive_terrain_topology(&map, &face_topo).expect("derive terrain topology");

        assert_eq!(face_topo.faces.len(), map.tiles.len());
        assert_eq!(derived.triangles.len(), map.tiles.len() * 24);

        let mut src_to_incidents: HashMap<usize, Vec<HexCoord>> = HashMap::new();
        for face in &face_topo.faces {
            for &v_id in &face.vertices {
                src_to_incidents
                    .entry(v_id.index())
                    .or_default()
                    .push(face.hex);
            }
        }

        let mut displaced_count = 0;
        for (v_idx, src_vert) in face_topo.vertices.iter().enumerate() {
            let mut expected_incidents = src_to_incidents.get(&v_idx).cloned().unwrap_or_default();
            expected_incidents.sort_by_key(|c| (c.q, c.r));
            expected_incidents.dedup();

            let matches: Vec<_> = derived
                .vertices_xz
                .iter()
                .enumerate()
                .filter(|(_, &pos)| {
                    pos.to_array().map(f32::to_bits)
                        == src_vert.position.to_array().map(f32::to_bits)
                })
                .collect();

            assert_eq!(
                matches.len(),
                1,
                "Source vertex {v_idx} must match exactly 1 derived vertex, found {}",
                matches.len()
            );
            let (d_idx, &d_pos) = matches[0];

            assert_eq!(
                d_pos, src_vert.position,
                "Derived vertex {d_idx} position must match source vertex {v_idx} bit-identically"
            );

            assert_eq!(
                derived.vertex_influences[d_idx], expected_incidents,
                "Derived vertex {d_idx} influences must match actual incident faces for source vertex {v_idx}"
            );

            let reg_pos = regular_corner_position(src_vert.canonical_key)
                .expect("regular position for canonical key");
            if src_vert.position.to_array().map(f32::to_bits)
                != reg_pos.to_array().map(f32::to_bits)
            {
                displaced_count += 1;
            }
        }

        assert!(
            displaced_count > 0,
            "At least one source corner must be displaced from its regular canonical position"
        );
    }

    #[test]
    fn adapter_invariants_and_winding_hold() {
        let map = canonical_40x40_map();
        let face_topo = generate_hex_face_topology_with_profile(
            &map,
            WorldSeed::new(42),
            HexDeformationProfile::Organic,
        )
        .expect("generate face topology");

        let derived = derive_terrain_topology(&map, &face_topo).expect("derive terrain topology");

        for (i, tri) in derived.triangles.iter().enumerate() {
            let p0 = derived.vertices_xz[tri[0] as usize];
            let p1 = derived.vertices_xz[tri[1] as usize];
            let p2 = derived.vertices_xz[tri[2] as usize];

            assert!(p0.x.is_finite() && p0.y.is_finite());
            assert!(p1.x.is_finite() && p1.y.is_finite());
            assert!(p2.x.is_finite() && p2.y.is_finite());

            let area = 0.5 * ((p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y));
            assert!(
                area > 1e-6,
                "Triangle {i} area must be strictly positive (> 1e-6) enforcing positive winding"
            );
        }

        for influences in &derived.vertex_influences {
            let mut sorted = influences.clone();
            sorted.sort_by_key(|c| (c.q, c.r));
            sorted.dedup();
            assert_eq!(
                influences, &sorted,
                "Influences must be sorted and deduplicated"
            );
        }

        // Insertion order determinism test: Normal vs Reversed vs LCG-shuffled
        let tiles_vec: Vec<_> = map.tiles.clone().into_iter().collect();

        // Reversed map
        let mut reversed_map = map.clone();
        let mut rev_tiles = tiles_vec.clone();
        rev_tiles.reverse();
        reversed_map.tiles = rev_tiles.into_iter().collect();

        let rev_face_topo = generate_hex_face_topology_with_profile(
            &reversed_map,
            WorldSeed::new(42),
            HexDeformationProfile::Organic,
        )
        .expect("reversed face topology");
        let rev_derived =
            derive_terrain_topology(&reversed_map, &rev_face_topo).expect("reversed derive");

        assert_eq!(derived.vertices_xz, rev_derived.vertices_xz);
        assert_eq!(derived.triangles, rev_derived.triangles);
        assert_eq!(derived.triangle_cells, rev_derived.triangle_cells);
        assert_eq!(derived.vertex_influences, rev_derived.vertex_influences);

        // LCG Shuffled map
        let mut shuffled_map = map.clone();
        let mut shuf_tiles = tiles_vec;
        let mut lcg_state = 123456789u64;
        for i in (1..shuf_tiles.len()).rev() {
            let j = (lcg_rand(&mut lcg_state) as usize) % (i + 1);
            shuf_tiles.swap(i, j);
        }
        shuffled_map.tiles = shuf_tiles.into_iter().collect();

        let shuf_face_topo = generate_hex_face_topology_with_profile(
            &shuffled_map,
            WorldSeed::new(42),
            HexDeformationProfile::Organic,
        )
        .expect("shuffled face topology");
        let shuf_derived =
            derive_terrain_topology(&shuffled_map, &shuf_face_topo).expect("shuffled derive");

        assert_eq!(derived.vertices_xz, shuf_derived.vertices_xz);
        assert_eq!(derived.triangles, shuf_derived.triangles);
        assert_eq!(derived.triangle_cells, shuf_derived.triangle_cells);
        assert_eq!(derived.vertex_influences, shuf_derived.vertex_influences);
    }

    #[test]
    fn invalid_source_face_returns_typed_error() {
        let map = canonical_40x40_map();
        let mut face_topo = generate_hex_face_topology_with_profile(
            &map,
            WorldSeed::new(42),
            HexDeformationProfile::Subtle,
        )
        .expect("generate face topology");

        let bad_hex = HexCoord::new(0, 0);
        face_topo.hex_to_face.insert(bad_hex, FaceId::new(999999));

        let res = derive_terrain_topology(&map, &face_topo);
        assert!(
            matches!(res, Err(TerrainTopologyError::InvalidSourceFace { hex, face }) if hex == bad_hex && face == FaceId::new(999999))
        );
    }

    #[test]
    #[ignore = "Stage-2 4608 topology integration proof matrix"]
    fn full_face_to_terrain_topology_integration_matrix() {
        let mut verified_count = 0;
        for profile in q::all_profiles() {
            for (_, map) in q::all_shapes() {
                for seed in 0..256 {
                    let face_topo = generate_hex_face_topology_with_profile(
                        &map,
                        WorldSeed::new(seed),
                        profile,
                    )
                    .expect("face topo");
                    let derived = derive_terrain_topology(&map, &face_topo).expect("derive");

                    assert_eq!(derived.triangles.len(), map.tiles.len() * 24);

                    for tri in &derived.triangles {
                        let p0 = derived.vertices_xz[tri[0] as usize];
                        let p1 = derived.vertices_xz[tri[1] as usize];
                        let p2 = derived.vertices_xz[tri[2] as usize];
                        let area =
                            0.5 * ((p1.x - p0.x) * (p2.y - p0.y) - (p2.x - p0.x) * (p1.y - p0.y));
                        assert!(area > 1e-6, "Consistent positive area winding required");
                    }

                    for influences in &derived.vertex_influences {
                        let mut sorted = influences.clone();
                        sorted.sort_by_key(|c| (c.q, c.r));
                        sorted.dedup();
                        assert_eq!(influences, &sorted);
                    }

                    verified_count += 1;
                }
            }
        }
        assert_eq!(verified_count, 4_608);
    }
}
