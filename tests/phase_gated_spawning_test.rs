// tests/phase_gated_spawning_test.rs
use bevy::prelude::*;
use savage_fantasy::game_state::{EditorPhase, FactionManager, GameState};
use savage_fantasy::map::mines::MineDeposit;
use savage_fantasy::map::{FactionMarker, GenerateMapEvent, MapData};

#[test]
fn test_phase_gated_spawning_and_validation() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.init_state::<GameState>();
    app.init_state::<EditorPhase>();
    app.insert_resource(MapData::default());
    app.insert_resource(FactionManager::default());
    app.add_message::<GenerateMapEvent>();

    // Set state to Editing and Shape phase
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Editing);
    app.world_mut()
        .resource_mut::<NextState<EditorPhase>>()
        .set(EditorPhase::Shape);
    app.update();

    let map_data = app.world().resource::<MapData>();
    assert!(
        map_data.validation_errors.is_empty(),
        "Shape phase must have zero validation errors"
    );

    // Verify 0 mines exist on Shape phase
    let mine_count = app
        .world_mut()
        .query::<(Entity, &MineDeposit)>()
        .iter(app.world())
        .count();
    assert_eq!(mine_count, 0, "No mines should exist on Shape phase");

    // Verify 0 factions exist on Shape phase
    let faction_count = app
        .world_mut()
        .query::<(Entity, &FactionMarker)>()
        .iter(app.world())
        .count();
    assert_eq!(faction_count, 0, "No factions should exist on Shape phase");
}
