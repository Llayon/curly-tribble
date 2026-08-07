//! Integration tests for HexFaceTopology -> TerrainTopology adapter.
#[cfg(test)]
mod tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::corner_key::{canonical_corner_key, regular_corner_position};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::topology::derive_terrain_topology;
    use crate::map::{HexCoord, WorldSeed};

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

        let mut displaced_count = 0;
        for face in &face_topo.faces {
            for i in 0..6 {
                let v_id = face.vertices[i];
                let src_pos = face_topo.vertices[v_id.index()].position;
                let reg_pos =
                    regular_corner_position(canonical_corner_key(face.hex, i)).expect("reg pos");
                if src_pos != reg_pos {
                    displaced_count += 1;
                }
            }
        }
        assert!(
            displaced_count > 0,
            "At least one source corner must be displaced from regular position"
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
            assert!(area.is_finite(), "Triangle {i} area must be finite");
            assert!(
                area.abs() > 1e-6,
                "Triangle {i} area must be non-zero (non-degenerate)"
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

        // Insertion order independence test
        let mut shuffled_map = map.clone();
        let mut tiles_vec: Vec<_> = map.tiles.into_iter().collect();
        tiles_vec.reverse();
        shuffled_map.tiles = tiles_vec.into_iter().collect();

        let shuffled_face_topo = generate_hex_face_topology_with_profile(
            &shuffled_map,
            WorldSeed::new(42),
            HexDeformationProfile::Organic,
        )
        .expect("shuffled face topology");

        let shuffled_derived =
            derive_terrain_topology(&shuffled_map, &shuffled_face_topo).expect("shuffled derive");

        assert_eq!(derived.vertices_xz, shuffled_derived.vertices_xz);
        assert_eq!(derived.triangles, shuffled_derived.triangles);
        assert_eq!(derived.triangle_cells, shuffled_derived.triangle_cells);
        assert_eq!(
            derived.vertex_influences,
            shuffled_derived.vertex_influences
        );
    }

    #[test]
    #[ignore = "Stage-2 3072 topology integration proof matrix"]
    fn full_face_to_terrain_topology_integration_matrix() {
        let mut verified_count = 0;
        for profile in [
            HexDeformationProfile::Subtle,
            HexDeformationProfile::Organic,
        ] {
            for shape_size in [1, 2, 3, 5, 10, 40] {
                let mut map = MapData::default();
                for r in 0..shape_size {
                    let offset = r >> 1;
                    for q in -offset..(shape_size - offset) {
                        map.tiles.insert(HexCoord::new(q, r), TileData::default());
                    }
                }
                map.width = shape_size as u32;
                map.height = shape_size as u32;

                for seed in 0..256 {
                    let face_topo = generate_hex_face_topology_with_profile(
                        &map,
                        WorldSeed::new(seed),
                        profile,
                    )
                    .expect("face topo");
                    let derived = derive_terrain_topology(&map, &face_topo).expect("derive");
                    assert_eq!(derived.triangles.len(), map.tiles.len() * 24);
                    verified_count += 1;
                }
            }
        }
        assert_eq!(verified_count, 3_072);
    }
}
