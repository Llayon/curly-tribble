use bevy::prelude::*;
use savage_fantasy::map::river_gen::apply_mud_banks;
use savage_fantasy::map::zoning::{MapData, TerrainType, TileData};

#[test]
fn test_apply_mud_banks() {
    let mut map_data = MapData {
        width: 4,
        height: 4,
        tiles: vec![TileData::default(); 16],
    };
    // Set up a 4x4 map with water in the middle
    // (x, z) ranges from -2 to 1
    // (0,0) is water, neighbors should become mud
    if let Some(tile) = map_data.get_tile_mut(0, 0) {
        tile.terrain = TerrainType::Water;
    }
    // Set a neighbor to Grass
    if let Some(tile) = map_data.get_tile_mut(1, 1) {
        tile.terrain = TerrainType::Grass;
    }

    apply_mud_banks(&mut map_data);

    let tile = map_data.get_tile(1, 1).expect("Tile (1,1) should exist");
    assert_eq!(
        tile.terrain,
        TerrainType::Mud,
        "Grass neighbor of water should be mud"
    );
}
