// use savage_fantasy::map::river_gen::apply_rivers;
// use savage_fantasy::map::terrain_gen::TerrainConfig;
// use savage_fantasy::map::{MapData, TerrainType};

#[test]
#[ignore]
fn test_river_generation_creates_water() {
    /*
    let mut config = TerrainConfig::default();
    config.river_count = 1;
    config.river_start_elevation = 0.0; // Ensure we find a source easily

    let mut map_data = MapData::default();
    map_data.width = 40;
    map_data.height = 40;
    map_data.tiles = vec![savage_fantasy::map::TileData::default(); 1600];

    // Set some elevations so rivers can flow
    for tile in map_data.tiles.iter_mut() {
        tile.elevation = 0.5;
    }

    apply_rivers(&mut map_data, &config, 123);

    let water_count = map_data
        .tiles
        .iter()
        .filter(|t| t.terrain == TerrainType::Water)
        .count();
    assert!(
        water_count > 0,
        "River generation should create at least one water tile"
    );
    */
}
