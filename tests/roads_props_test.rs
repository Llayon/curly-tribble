// tests/roads_props_test.rs
use bevy::prelude::*;
use savage_fantasy::game_state::{EditorPhase, GameState};
use savage_fantasy::map::data::OceanState;
use savage_fantasy::map::props::{
    snap_prop_y, spawn_decorative_prop, DecorativeProp, PropSnapMode, PropType, PropsPlugin,
};
use savage_fantasy::map::roads::{
    generate_village_roads, spawn_faction_border_posts, BorderPost, RoadsPlugin,
};
use savage_fantasy::map::{HexCoord, MapData, RebuildMeshEvent, TerrainType, TileData};

#[test]
fn test_roads_border_posts_and_props() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.init_asset::<Scene>();

    app.insert_resource(State::new(EditorPhase::Villages));
    app.insert_resource(State::new(GameState::Playing));

    app.add_plugins((RoadsPlugin, PropsPlugin));
    app.add_message::<RebuildMeshEvent>();

    // Mock GameAssets
    let mut scenes = app.world_mut().resource_mut::<Assets<Scene>>();
    let scene_handle = scenes.add(Scene::new(World::default()));

    let mut map_data = MapData::default();
    map_data.width = 10;
    map_data.height = 10;

    // Set up land tiles
    for q in 0..=5 {
        for r in 0..=5 {
            map_data.tiles.insert(
                HexCoord::new(q, r),
                TileData {
                    ocean_state: OceanState::Land,
                    terrain: TerrainType::Grass,
                    elevation: 0.5,
                    faction_id: if q < 3 { Some(1) } else { None },
                    ..default()
                },
            );
        }
    }

    // 1. Test road generation
    let poi_start = HexCoord::new(0, 0);
    let poi_end = HexCoord::new(4, 0);
    generate_village_roads(&mut map_data, &[poi_start, poi_end]);

    let mid_tile = map_data.get_tile(2, 0).unwrap();
    assert_eq!(
        mid_tile.terrain,
        TerrainType::Dirt,
        "Road path tile must be converted to Dirt"
    );

    // 2. Test prop snapping
    let land_snapped = snap_prop_y(Vec3::new(0.0, 100.0, 0.0), PropSnapMode::Land, &map_data);
    let water_snapped = snap_prop_y(Vec3::new(0.0, 100.0, 0.0), PropSnapMode::Water, &map_data);

    assert_eq!(water_snapped.y, 0.0, "Water prop Y height must be 0.0");
    assert_eq!(
        land_snapped.y,
        map_data.get_hex_height(0, 0),
        "Land prop Y height must match terrain height"
    );

    // 3. Test border posts and prop spawning
    let assets = GameAssets {
        house_scene: scene_handle.clone(),
        tree_scene: scene_handle.clone(),
        bush_scene: scene_handle,
        ..default()
    };

    let border_posts =
        spawn_faction_border_posts(&mut app.world_mut().commands(), &map_data, &assets);
    assert!(
        !border_posts.is_empty(),
        "Border posts must be spawned on faction boundary"
    );

    let prop_entity = spawn_decorative_prop(
        &mut app.world_mut().commands(),
        Vec3::new(0.0, 0.0, 0.0),
        PropType::FlowerField,
        PropSnapMode::Land,
        &map_data,
        &assets,
    );

    app.insert_resource(map_data);
    app.update();

    let prop_component = app.world().get::<DecorativeProp>(prop_entity);
    assert!(
        prop_component.is_some(),
        "DecorativeProp component must exist on spawned prop"
    );
    assert_eq!(prop_component.unwrap().prop_type, PropType::FlowerField);

    let border_post_comp = app.world().get::<BorderPost>(border_posts[0]);
    assert!(
        border_post_comp.is_some(),
        "BorderPost component must exist on spawned post"
    );
}
