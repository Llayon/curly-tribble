/// Focused corruption tests for exact topology invariants.
#[cfg(test)]
mod mutation_tests {
    use crate::map::data::{MapData, TileData};
    use crate::map::face_topology::generator::generate_hex_face_topology;
    use crate::map::face_topology::types::{FaceId, HalfEdgeId, HexFaceTopology};
    use crate::map::face_topology::validate_complete_topology;
    use crate::map::{HexCoord, WorldSeed};

    fn two_hex_topology() -> (MapData, HexFaceTopology) {
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());
        map.tiles.insert(HexCoord::new(1, 0), TileData::default());
        let topology =
            generate_hex_face_topology(&map, WorldSeed::new(42)).expect("valid two-hex topology");
        (map, topology)
    }

    fn isolated_pair_topology() -> (MapData, HexFaceTopology) {
        let mut map = MapData::default();
        map.tiles.insert(HexCoord::new(0, 0), TileData::default());
        map.tiles.insert(HexCoord::new(5, 0), TileData::default());
        let topology =
            generate_hex_face_topology(&map, WorldSeed::new(42)).expect("valid isolated topology");
        (map, topology)
    }

    fn internal_edges(topology: &HexFaceTopology) -> (HalfEdgeId, HalfEdgeId) {
        let Some((index, edge)) = topology
            .half_edges
            .iter()
            .enumerate()
            .find(|(_, edge)| edge.twin.is_some())
        else {
            panic!("two-hex topology must have an internal edge");
        };
        (
            HalfEdgeId::new(index),
            edge.twin.unwrap_or(HalfEdgeId::new(0)),
        )
    }

    fn assert_rejected(topology: HexFaceTopology, map: &MapData, expected: &str) {
        let result = validate_complete_topology(&topology, map);
        let Err(error) = result else {
            panic!("corrupted topology unexpectedly validated: {topology:?}");
        };
        let message = format!("{error:?}");
        assert!(
            message.contains(expected),
            "expected '{expected}' in validation error, got {message}"
        );
    }

    #[test]
    fn rejects_one_missing_internal_twin() {
        let (map, mut topology) = two_hex_topology();
        let (edge_a, _) = internal_edges(&topology);
        topology.half_edges[edge_a.index()].twin = None;
        assert_rejected(topology, &map, "one missing twin");
    }

    #[test]
    fn rejects_asymmetric_twin_links() {
        let (map, mut topology) = two_hex_topology();
        let (_, edge_b) = internal_edges(&topology);
        topology.half_edges[edge_b.index()].twin = Some(edge_b);
        assert_rejected(topology, &map, "asymmetric twins");
    }

    #[test]
    fn rejects_same_direction_twin_endpoints() {
        let (map, mut topology) = two_hex_topology();
        let (edge_a, edge_b) = internal_edges(&topology);
        let origin = topology.half_edges[edge_a.index()].origin;
        let destination = topology.half_edges[edge_a.index()].destination;
        topology.half_edges[edge_b.index()].origin = origin;
        topology.half_edges[edge_b.index()].destination = destination;
        assert_rejected(topology, &map, "are not reversed");
    }

    #[test]
    fn rejects_logical_neighbors_with_extra_shared_vertex() {
        let (map, mut topology) = two_hex_topology();
        let face_a = topology.faces[0].vertices;
        let face_b_id = FaceId::new(1);
        let Some(third_vertex) = face_a
            .into_iter()
            .find(|vertex| !topology.faces[face_b_id.index()].vertices.contains(vertex))
        else {
            panic!("two faces must have a non-shared vertex");
        };
        let face_b = &mut topology.faces[face_b_id.index()];
        let Some(replace_index) = face_b
            .vertices
            .iter()
            .position(|vertex| !face_a.contains(vertex))
        else {
            panic!("two faces must have a non-shared vertex");
        };
        face_b.vertices[replace_index] = third_vertex;
        assert_rejected(topology, &map, "share 3 vertices");
    }

    #[test]
    fn rejects_logical_neighbors_with_two_shared_half_edges() {
        let (map, mut topology) = two_hex_topology();
        let (edge_a, edge_b) = internal_edges(&topology);
        let shared_origin = topology.half_edges[edge_b.index()].origin;
        let shared_destination = topology.half_edges[edge_b.index()].destination;
        let face_b = topology.half_edges[edge_b.index()].incident_face;
        let Some(extra_edge_index) = topology
            .half_edges
            .iter()
            .enumerate()
            .find(|(index, edge)| edge.incident_face == face_b && HalfEdgeId::new(*index) != edge_b)
            .map(|(index, _)| index)
        else {
            panic!("two-hex face must have another edge");
        };
        topology.half_edges[extra_edge_index].origin = shared_origin;
        topology.half_edges[extra_edge_index].destination = shared_destination;
        let _ = edge_a;
        assert_rejected(topology, &map, "shared half-edges");
    }

    #[test]
    fn rejects_non_neighboring_faces_sharing_an_edge() {
        let (map, mut topology) = isolated_pair_topology();
        let first_face = topology.faces[0].vertices;
        let second_face = &mut topology.faces[1];
        second_face.vertices[0] = first_face[0];
        second_face.vertices[1] = first_face[1];
        assert_rejected(topology, &map, "Non-neighbor faces");
    }

    #[test]
    fn rejects_internal_edge_marked_as_boundary() {
        let (map, mut topology) = two_hex_topology();
        let (edge_a, edge_b) = internal_edges(&topology);
        topology.half_edges[edge_a.index()].twin = None;
        topology.half_edges[edge_b.index()].twin = None;
        assert_rejected(topology, &map, "incorrectly boundary");
    }

    #[test]
    fn rejects_border_edge_linked_to_unrelated_face() {
        let (map, mut topology) = isolated_pair_topology();
        let Some(edge_a) = topology
            .half_edges
            .iter()
            .enumerate()
            .find(|(_, edge)| edge.incident_face == FaceId::new(0) && edge.twin.is_none())
            .map(|(index, _)| HalfEdgeId::new(index))
        else {
            panic!("first isolated face must have a border edge");
        };
        let Some(edge_b) = topology
            .half_edges
            .iter()
            .enumerate()
            .find(|(_, edge)| edge.incident_face == FaceId::new(1) && edge.twin.is_none())
            .map(|(index, _)| HalfEdgeId::new(index))
        else {
            panic!("second isolated face must have a border edge");
        };
        topology.half_edges[edge_a.index()].twin = Some(edge_b);
        assert_rejected(topology, &map, "non-neighbor faces");
    }
}
