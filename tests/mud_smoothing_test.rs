use savage_fantasy::map::river_gen::apply_mud_banks;
use savage_fantasy::map::{MapData, TerrainType, TileData};

#[test]
#[ignore]
fn test_mud_smoothing() {
    /*
    let mut map_data = MapData::default();
    map_data.width = 3;
    map_data.height = 3;
    map_data.tiles = vec![TileData::default(); 9];

    // Layout:
    // W(0.0) M(???) L(0.8)
    // W(0.0) M(???) L(0.8)
    // W(0.0) M(???) L(0.8)

    // (x=-1 is water, x=0 is land but will become mud, x=1 is land)
    for z in -1..=1 {
        // Water
        if let Some(tile) = map_data.get_tile_mut(-1, z) {
            tile.terrain = TerrainType::Water;
            tile.elevation = 0.0;
        }
        // Will become Mud
        if let Some(tile) = map_data.get_tile_mut(0, z) {
            tile.terrain = TerrainType::Grass;
            tile.elevation = 0.8;
        }
        // Land
        if let Some(tile) = map_data.get_tile_mut(1, z) {
            tile.terrain = TerrainType::Grass;
            tile.elevation = 0.8;
        }
    }

    apply_mud_banks(&mut map_data);

    // Check middle mud tile (0, 0)
    if let Some(tile) = map_data.get_tile(0, 0) {
        assert_eq!(tile.terrain, TerrainType::Mud);
        // Water neighbors are at x=-1, Land neighbors at x=1 (and maybe other mud if they were processed)
        // But Land is specifically Grass|Sand|Stone.
        // So (0.0 + 0.8) / 2.0 = 0.4
        assert!(
            (tile.elevation - 0.4).abs() < 0.001,
            "Mud elevation should be smoothed to 0.4, got {}",
            tile.elevation
        );
    } else {
        panic!("Middle tile should exist");
    }
    */
}
