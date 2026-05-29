// use bevy::prelude::*;
// use savage_fantasy::map::navigation::{COST_BASE, COST_BLOCKER};
// use savage_fantasy::map::{
//     MapData, TerrainType, TileData,
// };

#[test]
#[ignore]
fn test_slope_detection() {
    /*
    let mut map_data = MapData {
        width: 3,
        height: 3,
        tiles: vec![TileData::default(); 9],
    };

    // Center tile at (0,0) - grid coordinates are relative to center
    // half_w = 1, half_h = 1. x range: -1..1, z range: -1..1
    // ux = x + 1, uz = z + 1

    // Set elevation 0.0 for center
    if let Some(tile) = map_data.get_tile_mut(0, 0) {
        tile.elevation = 0.0;
        tile.terrain = TerrainType::Grass;
    }

    // Set elevation 0.4 for neighbor (1, 0) -> Slope 0.4 > 0.3
    if let Some(tile) = map_data.get_tile_mut(1, 0) {
        tile.elevation = 0.4;
        tile.terrain = TerrainType::Grass;
    }

    assert!(
        map_data.is_too_steep(0, 0),
        "Center should be steep due to (1,0)"
    );
    assert!(
        map_data.is_too_steep(1, 0),
        "(1,0) should be steep due to (0,0)"
    );
    assert!(!map_data.is_too_steep(-1, 0), "(-1,0) should not be steep");
    */
}
