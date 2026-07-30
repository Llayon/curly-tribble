// tests/balance_test.rs
use bevy::prelude::*;
use savage_fantasy::economy::assets::GameAssets;
use savage_fantasy::game_state::{EditorPhase, GameState};
use savage_fantasy::map::balance::BalancePlugin;
use savage_fantasy::map::balance_commands::{AutoBalanceCommand, BalanceCommandsPlugin};
use savage_fantasy::map::{ForestType, HexCoord, MapData, RebuildMeshEvent, TerrainType, TileData};

#[test]
fn test_balance_validation_and_command() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.init_asset::<Scene>();

    app.insert_resource(State::new(EditorPhase::Balance));
    app.insert_resource(State::new(GameState::Playing));

    // Register our events/messages and plugins
    app.add_plugins((BalancePlugin, BalanceCommandsPlugin));
    app.add_message::<RebuildMeshEvent>();

    // Mock GameAssets
    let mut scenes = app.world_mut().resource_mut::<Assets<Scene>>();
    let bush_scene = scenes.add(Scene::new(World::default()));

    app.insert_resource(GameAssets {
        bush_scene,
        ..default()
    });

    // Create MapData with vacant land tiles inside Faction 1's territory
    let mut map_data = MapData::default();
    map_data.width = 10;
    map_data.height = 10;

    for q in -3..=3 {
        for r in -3..=3 {
            map_data.tiles.insert(
                HexCoord::new(q, r),
                TileData {
                    faction_id: Some(1),
                    ocean_state: savage_fantasy::map::data::OceanState::Land,
                    terrain: TerrainType::Grass,
                    forest_type: ForestType::None,
                    forest_density: 0.0,
                    ..default()
                },
            );
        }
    }

    app.insert_resource(map_data);

    // 1. Initial validation - should be deficient in Wood, Food, Flax
    // We run the schedule once so that validate_starter_resources runs.
    app.update();

    {
        let map_res = app.world().resource::<MapData>();
        assert!(!map_res.validation_errors.is_empty());
        let errors = &map_res.validation_errors;
        assert!(errors[0].contains("deficient in Wood/Food/Flax"));
    }

    // 2. Run the AutoBalanceCommand
    {
        let mut map_res = app.world_mut().resource_mut::<MapData>();
        map_res.validation_errors.clear();
    }
    app.world_mut()
        .commands()
        .queue(AutoBalanceCommand { faction_id: 1 });

    // We update again to apply the command and run the validation system
    app.update();

    // 3. Verify that deficiencies are resolved, and validation errors are empty
    {
        let map_res = app.world().resource::<MapData>();
        assert!(
            map_res.validation_errors.is_empty(),
            "Validation errors: {:?}",
            map_res.validation_errors
        );
    }
}
