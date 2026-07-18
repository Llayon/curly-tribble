use bevy::prelude::*;
use savage_fantasy::game_state::{EditorPhase, GameState};
use savage_fantasy::map::treasures::{MapToTarget, Targeting, TreasureDeposit, VisibleTreasure};
use savage_fantasy::map::HexCoord;

#[test]
fn test_treasure_link_creation() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(State::new(EditorPhase::Treasures));
    app.insert_resource(State::new(GameState::Playing));

    // Spawn Source
    let source = app
        .world_mut()
        .spawn((
            TreasureDeposit {
                contents: vec![],
                hex_coord: HexCoord::new(0, 0),
            },
            VisibleTreasure,
        ))
        .id();

    // Spawn Target
    let target = app
        .world_mut()
        .spawn((
            TreasureDeposit {
                contents: vec![],
                hex_coord: HexCoord::new(1, 0),
            },
            VisibleTreasure,
        ))
        .id();

    // Simulate Link (Semantic Graph Pattern)
    app.world_mut().entity_mut(source).with_children(|parent| {
        parent.spawn((MapToTarget, Targeting(target)));
    });

    // Verify
    let mut q_links = app.world_mut().query::<(&ChildOf, &Targeting)>();
    let (child_of, targeting) = q_links.single(app.world()).unwrap();

    assert_eq!(child_of.0, source);
    assert_eq!(targeting.0, target);
}
