// tests/editor_state_test.rs
use bevy::prelude::*;
use savage_fantasy::economy::GameAssets;
use savage_fantasy::game_state::{EditorPhase, GameState};
use savage_fantasy::map::MapData;
use savage_fantasy::pawn::{despawn_settlers, spawn_starting_settler, Settler};

#[test]
fn test_editor_state_and_playtest_settler_lifecycle() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::state::app::StatesPlugin));
    app.init_state::<GameState>();
    app.init_state::<EditorPhase>();
    app.insert_resource(MapData::default());
    app.insert_resource(GameAssets::default());

    app.add_systems(OnEnter(GameState::Playing), spawn_starting_settler);
    app.add_systems(OnExit(GameState::Playing), despawn_settlers);

    // Initial state is Loading, transitioning to Editing
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Editing);
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Editing
    );

    // 1. In Editing mode, zero settlers exist
    let mut q_settlers = app.world_mut().query::<(Entity, &Settler)>();
    assert_eq!(
        q_settlers.iter(app.world()).count(),
        0,
        "No settlers should exist in Editing state"
    );

    // 2. Transition to Playing (Playtest Mode)
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Playing);
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Playing
    );

    // Settler spawned on entering Playing state
    let count = app
        .world_mut()
        .query::<(Entity, &Settler)>()
        .iter(app.world())
        .count();
    assert_eq!(count, 1, "Settler must spawn on entering Playing state");

    // 3. Transition back to Editing
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::Editing);
    app.update();

    assert_eq!(
        *app.world().resource::<State<GameState>>().get(),
        GameState::Editing
    );

    // Settler cleaned up on exiting Playing state
    let count_after = app
        .world_mut()
        .query::<(Entity, &Settler)>()
        .iter(app.world())
        .count();
    assert_eq!(
        count_after, 0,
        "Settler must despawn on exiting Playing state"
    );
}
