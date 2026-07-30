// tests/subhex_buildings_test.rs
use bevy::prelude::*;
use savage_fantasy::economy::assets::GameAssets;
use savage_fantasy::game_state::{EditorPhase, GameState};
use savage_fantasy::map::buildings::{
    flatten_building_foundation, spawn_building_structure, BuildingStructure, BuildingType,
    BuildingsPlugin,
};
use savage_fantasy::map::subhex::{snap_to_terrain_y, SubHexPlacementPlugin};
use savage_fantasy::map::{HexCoord, MapData, RebuildMeshEvent, TileData};

#[test]
fn test_subhex_snapping_and_building_flattening() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.init_asset::<Scene>();

    app.insert_resource(State::new(EditorPhase::Buildings));
    app.insert_resource(State::new(GameState::Playing));

    app.add_plugins((SubHexPlacementPlugin, BuildingsPlugin));
    app.add_message::<RebuildMeshEvent>();

    // Mock GameAssets
    let mut scenes = app.world_mut().resource_mut::<Assets<Scene>>();
    let house_scene = scenes.add(Scene::new(World::default()));

    app.insert_resource(GameAssets {
        house_scene: house_scene.clone(),
        tree_scene: house_scene.clone(),
        bush_scene: house_scene.clone(),
        ..default()
    });

    let mut map_data = MapData::default();
    map_data.width = 10;
    map_data.height = 10;

    // Create uneven terrain around (0,0)
    map_data.tiles.insert(
        HexCoord::new(0, 0),
        TileData {
            elevation: 0.5,
            ..default()
        },
    );
    map_data.tiles.insert(
        HexCoord::new(1, 0),
        TileData {
            elevation: 0.8,
            ..default()
        },
    );
    map_data.tiles.insert(
        HexCoord::new(0, 1),
        TileData {
            elevation: 0.2,
            ..default()
        },
    );

    // 1. Test snap_to_terrain_y
    let world_pos = Vec3::new(0.0, 100.0, 0.0);
    let snapped = snap_to_terrain_y(world_pos, &map_data);
    let expected_y = map_data.get_hex_height(0, 0);
    assert_eq!(snapped.y, expected_y);

    // 2. Test terrain flattening
    flatten_building_foundation(&mut map_data, HexCoord::new(0, 0), 1);
    let center_elev = map_data.get_tile(0, 0).unwrap().elevation;
    let neighbor_elev = map_data.get_tile(1, 0).unwrap().elevation;
    assert_eq!(
        center_elev, neighbor_elev,
        "Neighbor elevation must match center foundation"
    );

    // 3. Test building structure spawning
    let assets = GameAssets {
        house_scene: house_scene.clone(),
        tree_scene: house_scene.clone(),
        bush_scene: house_scene,
        ..default()
    };
    let building_entity = spawn_building_structure(
        &mut app.world_mut().commands(),
        HexCoord::new(0, 0),
        BuildingType::TradePost,
        1,
        &mut map_data,
        &assets,
    );
    app.insert_resource(map_data);

    app.update();

    let structure = app.world().get::<BuildingStructure>(building_entity);
    assert!(
        structure.is_some(),
        "BuildingStructure component must exist on spawned building"
    );
    assert_eq!(structure.unwrap().building_type, BuildingType::TradePost);
}
