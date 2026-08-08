// src/map/height_constraints/tests_cliffs.rs
//! Unit tests for cliff height constraint binding and lower-side preservation.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct HeightConstraintCliffsTestsPlugin;

impl Plugin for HeightConstraintCliffsTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::map::data::EdgeCoord;
    use crate::map::data::{CliffLowerSide, EdgeData, EdgeType, MapData, TileData, WorldSeed};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compiler::compile_height_constraints;
    use crate::map::height_constraints::types::HeightConstraintCompileError;
    use crate::map::surface_topology::generator::generate_surface_topology;
    use crate::map::HexCoord;

    #[test]
    fn cliff_binding_and_lower_side_preservation() {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(1, 0);
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());

        let edge_unresolved = EdgeCoord::new(c1, c2);
        map.edges.insert(
            edge_unresolved,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::Unresolved,
            },
        );

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Face topology failed");
        let surface = generate_surface_topology(&face_topology).expect("Surface topology failed");

        let constraints = compile_height_constraints(&map, &surface).unwrap();
        assert_eq!(constraints.cliffs.len(), 1);
        let cliff = &constraints.cliffs[0];

        assert_eq!(cliff.logical_edge, edge_unresolved);
        assert_eq!(cliff.lower_side, CliffLowerSide::Unresolved);
        assert_eq!(cliff.segments.len(), 2);

        for segment in &cliff.segments {
            let he_a = &surface.half_edges[segment.half_edge_a.index()];
            let he_b = &surface.half_edges[segment.half_edge_b.index()];

            assert_eq!(he_a.twin, Some(segment.half_edge_b));
            assert_eq!(he_b.twin, Some(segment.half_edge_a));

            let face_a = &surface.faces[he_a.incident_face.index()];
            let face_b = &surface.faces[he_b.incident_face.index()];

            assert_eq!(face_a.owner_hex, edge_unresolved.a);
            assert_eq!(face_b.owner_hex, edge_unresolved.b);
        }
    }

    #[test]
    fn semantic_edit_stability_lower_side_change() {
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
                cliff_lower_side: CliffLowerSide::Unresolved,
            },
        );

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Face topology failed");
        let surface = generate_surface_topology(&face_topology).expect("Surface topology failed");

        let constraints_unresolved = compile_height_constraints(&map, &surface).unwrap();

        if let Some(edge_data) = map.edges.get_mut(&edge) {
            edge_data.cliff_lower_side = CliffLowerSide::A;
        }

        let constraints_a = compile_height_constraints(&map, &surface).unwrap();

        assert_eq!(
            constraints_a.cliffs[0].segments,
            constraints_unresolved.cliffs[0].segments
        );
        assert_eq!(constraints_a.cliffs[0].lower_side, CliffLowerSide::A);
    }

    #[test]
    fn non_adjacent_cliff_edge_fails_with_typed_error() {
        let mut map = MapData::default();
        let c1 = HexCoord::new(0, 0);
        let c2 = HexCoord::new(5, 5); // Non-adjacent
        map.tiles.insert(c1, TileData::default());
        map.tiles.insert(c2, TileData::default());

        let invalid_edge = EdgeCoord::new(c1, c2);
        map.edges.insert(
            invalid_edge,
            EdgeData {
                edge_type: EdgeType::Cliff,
                cliff_lower_side: CliffLowerSide::Unresolved,
            },
        );

        let seed = WorldSeed::new(42);
        let face_topology =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("Face topology failed");
        let surface = generate_surface_topology(&face_topology).expect("Surface topology failed");

        let err = compile_height_constraints(&map, &surface).unwrap_err();
        assert_eq!(
            err,
            HeightConstraintCompileError::MissingSurfaceBoundary(invalid_edge)
        );
    }
}
