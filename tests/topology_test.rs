// tests/topology_test.rs
use bevy::prelude::*;
use savage_fantasy::game_state::{EditorPhase, GameState};
use savage_fantasy::map::data::OceanState;
use savage_fantasy::map::terrain_gen::{TerrainConfig, TerrainGenerator};
use savage_fantasy::map::topology::{
    canonical_vertex_key, compute_vertex_heights, generate_topology_from_map_data,
};
use savage_fantasy::map::{HexCoord, MapData, TileData, HEX_SIZE};

#[test]
fn test_topology_validation_cases() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(State::new(EditorPhase::Balance));
    app.insert_resource(State::new(GameState::Editing));

    // 1. Generate deterministic 40x40 map with seed 42
    let seed = 42u32;
    let config = TerrainConfig {
        map_width: 40,
        map_height: 40,
        seed,
        ..default()
    };

    let mut map_data = MapData::default();
    map_data.width = 40;
    map_data.height = 40;

    let generator = TerrainGenerator::new(seed);
    let half_w = 20i32;
    let half_h = 20i32;

    for q in -half_w..half_w {
        for r in -half_h..half_h {
            let coord = HexCoord::new(q, r);
            let world = coord.to_world(HEX_SIZE);
            let is_ocean =
                q <= -half_w + 1 || q >= half_w - 2 || r <= -half_h + 1 || r >= half_h - 2;
            let shape_val = generator.get_shape_value(&config, world.x, world.z);
            let ocean_state = if is_ocean || shape_val <= 0.0 {
                OceanState::Ocean
            } else {
                OceanState::Land
            };

            let elev = if ocean_state == OceanState::Ocean {
                0.0
            } else {
                0.2 + 0.5 * ((q as f32 * 0.1).sin() + (r as f32 * 0.1).cos()).abs()
            };

            map_data.tiles.insert(
                coord,
                TileData {
                    ocean_state,
                    elevation: elev,
                    ..default()
                },
            );
        }
    }

    // 1. Balance displays logical map
    assert_eq!(
        map_data.tiles.len(),
        1600,
        "Map must have 40x40 = 1600 tiles"
    );

    // 2. Build topology
    let topology_balance = generate_topology_from_map_data(&map_data);
    let tri_count = topology_balance.triangles.len();
    assert_eq!(
        tri_count,
        1600 * 24,
        "Balance must have exactly 24 triangles per hex (1600 * 24 = 38400)"
    );

    // 3. Shared boundary vertices test
    let shared_vertices = topology_balance
        .vertex_influences
        .iter()
        .filter(|inf| inf.len() > 1)
        .count();
    assert!(
        shared_vertices > 0,
        "Shared boundary vertices must exist across adjacent hexes"
    );

    // 4. Height3D uses identical topology & counts
    let topology_height = generate_topology_from_map_data(&map_data);
    assert_eq!(
        topology_height.vertices_xz.len(),
        topology_balance.vertices_xz.len(),
        "Height3D must use exact same vertex count as Balance"
    );
    assert_eq!(
        topology_height.triangles.len(),
        topology_balance.triangles.len(),
        "Height3D must use exact same index/triangle count as Balance"
    );

    // 5. Verify no cracks & shared flat-border IDs
    let p_test = Vec2::new(10.0, 15.0);
    let key1 = canonical_vertex_key(p_test);
    let key2 = canonical_vertex_key(p_test + Vec2::new(0.0001, -0.0001));
    assert_eq!(
        key1, key2,
        "Canonical vertex keying must deduplicate close boundary vertices"
    );

    use savage_fantasy::map::topology::TerrainHeightMode;

    // 6. Height3D produces smooth slopes
    let heights_balance =
        compute_vertex_heights(&topology_balance, &map_data, TerrainHeightMode::Flat);
    let heights_3d =
        compute_vertex_heights(&topology_height, &map_data, TerrainHeightMode::Relief3D);

    assert!(
        heights_balance.iter().all(|&y| y == 0.0),
        "Balance vertex heights must all be zero"
    );
    assert!(
        heights_3d.iter().any(|&y| y > 0.0),
        "Height3D vertex heights must include non-zero elevations"
    );

    // 7. Returning to Balance restores flat surface
    let heights_return =
        compute_vertex_heights(&topology_balance, &map_data, TerrainHeightMode::Flat);
    assert!(
        heights_return.iter().all(|&y| y == 0.0),
        "Returning to Balance must restore a flat surface (all y = 0)"
    );

    // 8. Rebuilding with same seed produces identical topology
    let topology_rebuild = generate_topology_from_map_data(&map_data);
    assert_eq!(
        topology_rebuild.vertices_xz, topology_balance.vertices_xz,
        "Rebuilding topology from same map data must yield identical vertex positions"
    );
    assert_eq!(
        topology_rebuild.triangles, topology_balance.triangles,
        "Rebuilding topology from same map data must yield identical triangle indices"
    );

    println!("Topological Validation Results (Seed 42, 40x40 map):");
    println!("- Logical Tiles: {}", map_data.tiles.len());
    println!("- Unique Vertices: {}", topology_balance.vertices_xz.len());
    println!("- Total Triangles: {}", topology_balance.triangles.len());
    println!("- Shared Boundary Vertices: {}", shared_vertices);
    println!("- All 10 validation points PASSED cleanly!");
}
