/// Focused tests for debug-only topology helpers.
#[cfg(test)]
mod debug_tests {
    use crate::game_state::EditorPhase;
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::debug::{
        debug_overlay_visible, extract_shared_vertices, extract_unique_undirected_edges,
        HexFaceDebugSettings,
    };
    use crate::map::face_topology::generator::generate_hex_face_topology;
    use crate::map::HexCoord;
    use crate::map::WorldSeed;
    use std::collections::HashSet;

    fn two_hex_map() -> MapData {
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());
        map.tiles.insert(HexCoord::new(1, 0), TileData::default());
        map
    }

    #[test]
    fn debug_settings_are_disabled_with_expected_submode_defaults() {
        let settings = HexFaceDebugSettings::default();
        assert!(!settings.enabled);
        assert!(settings.show_regular_outlines);
        assert!(settings.show_warped_outlines);
        assert!(!settings.show_shared_vertices);
        assert!(!settings.show_half_edge_directions);
    }

    #[test]
    fn unique_edges_and_vertices_use_authoritative_ids() {
        let map = two_hex_map();
        let topology =
            generate_hex_face_topology(&map, WorldSeed::new(42)).expect("two-hex topology");
        let edges = extract_unique_undirected_edges(&topology);
        let vertices = extract_shared_vertices(&topology);
        assert_eq!(
            edges.len(),
            topology.stats.paired_edge_count + topology.stats.border_edge_count
        );
        assert_eq!(vertices.len(), topology.vertices.len());
        assert_eq!(edges.iter().collect::<HashSet<_>>().len(), edges.len());
    }

    #[test]
    fn helpers_do_not_mutate_map_or_topology() {
        let map = two_hex_map();
        let topology =
            generate_hex_face_topology(&map, WorldSeed::new(42)).expect("two-hex topology");
        let map_keys_before: HashSet<_> = map.tiles.keys().copied().collect();
        let map_dimensions_before = (map.width, map.height);
        let topology_before = topology.clone();
        let _ = extract_unique_undirected_edges(&topology);
        let _ = extract_shared_vertices(&topology);
        assert_eq!(map_dimensions_before, (map.width, map.height));
        assert_eq!(map_keys_before, map.tiles.keys().copied().collect());
        assert_eq!(topology, topology_before);
    }

    #[test]
    fn visibility_is_disabled_or_limited_to_flat_phases() {
        let mut settings = HexFaceDebugSettings::default();
        assert!(!debug_overlay_visible(&settings, EditorPhase::Shape));
        settings.enabled = true;
        assert!(debug_overlay_visible(&settings, EditorPhase::Balance));
        assert!(!debug_overlay_visible(&settings, EditorPhase::Height3D));
    }
}
