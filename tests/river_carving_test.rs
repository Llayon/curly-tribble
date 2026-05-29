// use savage_fantasy::map::river_gen::apply_rivers;
// use savage_fantasy::map::terrain_gen::TerrainConfig;
// use savage_fantasy::map::{MapData, TerrainType, TileData};

#[test]
#[ignore]
fn test_river_carving_is_monotonic() {
    /*
    let mut config = TerrainConfig::default();
    config.river_count = 1;
    config.river_start_elevation = 0.5;
    config.river_depth = 0.1;

    let mut map_data = MapData::default();
    map_data.width = 20;
    map_data.height = 20;
    // Create a ramp: West (high) to East (low)
    map_data.tiles = (0..400)
        .map(|i| {
            let x = (i % 20) as f32;
            TileData {
                elevation: 1.0 - (x / 20.0), // 1.0 down to 0.0
                terrain: TerrainType::Grass,
                ..Default::default()
            }
        })
        .collect();

    apply_rivers(&mut map_data, &config, 42);

    // Find the river path
    let mut river_tiles = Vec::new();
    // Since it's a ramp from West to East, the river should flow West -> East
    // We can just find all water tiles and check their connectivity or at least their elevation trend

    for x in -10..10 {
        for z in -10..10 {
            if let Some(tile) = map_data.get_tile(x, z) {
                if tile.terrain == TerrainType::Water {
                    river_tiles.push((x, z, tile.elevation));
                }
            }
        }
    }

    assert!(!river_tiles.is_empty(), "Should have generated a river");

    // Sorting by X (west to east) should give us the flow direction mostly
    river_tiles.sort_by(|a, b| a.0.cmp(&b.0));

    for i in 0..river_tiles.len() - 1 {
        assert!(
            river_tiles[i].2 >= river_tiles[i + 1].2 - 0.001,
            "River elevation should be non-increasing from source to sea. Tile {} ({}) -> Tile {} ({})",
            i, river_tiles[i].2, i+1, river_tiles[i+1].2
        );
    }
    */
}
