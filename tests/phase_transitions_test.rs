use bevy::prelude::*;
use savage_fantasy::game_state::EditorPhase;
use savage_fantasy::map::artifacts::{Artifact, ArtifactBundle, ArtifactLocation};
use savage_fantasy::map::phase_transitions::{
    extract_artifacts_on_phase_change, PhaseTransitionsPlugin,
};
use savage_fantasy::map::treasures::{
    ArtifactType, ContainsArtifact, TreasureDeposit, TreasureItem,
};
use savage_fantasy::map::HexCoord;

#[test]
fn test_extract_artifacts_on_phase_change() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app.init_state::<EditorPhase>();
    app.add_plugins(PhaseTransitionsPlugin);

    // Initial state is probably not Artifacts.

    // Spawn a TreasureDeposit with ArtifactDef
    let deposit_entity = app
        .world_mut()
        .spawn(TreasureDeposit {
            contents: vec![
                TreasureItem::Gold(100),
                TreasureItem::ArtifactDef(ArtifactType::AncientRelic),
            ],
            hex_coord: HexCoord::new(0, 0),
        })
        .id();

    // Trigger state transition to Artifacts
    app.world_mut()
        .resource_mut::<NextState<EditorPhase>>()
        .set(EditorPhase::Artifacts);

    app.update(); // Process the transition

    // Check if ArtifactDef was converted to ArtifactRef
    let deposit = app.world().get::<TreasureDeposit>(deposit_entity).unwrap();
    assert_eq!(deposit.contents.len(), 2);
    assert_eq!(deposit.contents[0], TreasureItem::Gold(100));

    let artifact_ref = &deposit.contents[1];
    let artifact_entity = match artifact_ref {
        TreasureItem::ArtifactRef(entity) => *entity,
        _ => panic!("Expected ArtifactRef, got {:?}", artifact_ref),
    };

    // Check if ArtifactBundle was spawned
    let artifact = app
        .world()
        .get::<Artifact>(artifact_entity)
        .expect("Artifact missing");
    assert_eq!(artifact.artifact_type, ArtifactType::AncientRelic);
    assert_eq!(
        artifact.location,
        ArtifactLocation::InTreasure(deposit_entity)
    );

    // Check if ContainsArtifact child was spawned on the TreasureDeposit
    let children = app
        .world()
        .get::<Children>(deposit_entity)
        .expect("No children on deposit");
    let mut found = false;
    for child in children.iter() {
        if let Some(contains) = app.world().get::<ContainsArtifact>(child) {
            assert_eq!(contains.artifact, artifact_entity);
            found = true;
            break;
        }
    }
    assert!(found, "ContainsArtifact child missing");
}
