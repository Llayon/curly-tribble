use crate::game_state::EditorPhase;
use crate::map::{ForestType, LandscapeFeature, MapData, TerrainType};
use crate::map::{GenerateMapEvent, GenerationMode};
use bevy::prelude::*;

pub struct PhaseTransitionsPlugin;

impl Plugin for PhaseTransitionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(EditorPhase::Artifacts),
            extract_artifacts_on_phase_change,
        );
    }
}

pub fn extract_artifacts_on_phase_change(
    mut commands: Commands,
    mut q_treasures: Query<
        (Entity, &mut crate::map::TreasureDeposit),
        With<crate::map::TreasureDeposit>,
    >,
) {
    for (entity, mut deposit) in &mut q_treasures {
        let mut new_contents = Vec::new();
        for item in deposit.contents.drain(..) {
            if let crate::map::TreasureItem::ArtifactDef(a_type) = item {
                let artifact_entity = commands
                    .spawn(crate::map::artifacts::ArtifactBundle {
                        artifact: crate::map::artifacts::Artifact {
                            artifact_type: a_type,
                            location: crate::map::artifacts::ArtifactLocation::InTreasure,
                        },
                        name: Name::new(format!("{a_type:?} (Artifact)")),
                        marker: crate::map::MapEntity,
                        transform: Transform::default(),
                        global_transform: GlobalTransform::default(),
                        visibility: Visibility::default(),
                        inherited_visibility: InheritedVisibility::default(),
                        view_visibility: ViewVisibility::default(),
                    })
                    .id();

                commands
                    .entity(artifact_entity)
                    .insert(crate::map::artifacts::StoredInTreasure(entity));

                commands.entity(entity).with_children(|parent| {
                    parent.spawn(crate::map::treasures::ContainsArtifact(artifact_entity));
                });

                new_contents.push(crate::map::TreasureItem::ArtifactRef(artifact_entity));
            } else {
                new_contents.push(item);
            }
        }
        deposit.contents = new_contents;
    }
}

pub fn rebuild_map_on_phase_change(
    mut ev_gen: MessageWriter<GenerateMapEvent>,
    phase: Res<State<EditorPhase>>,
    map_data: Res<MapData>,
    q_pois: Query<&crate::map::PointOfInterest>,
    q_camps: Query<&crate::map::EnemyCamp>,
    q_deposits: Query<&crate::map::ResourceDeposit>,
    q_treasures: Query<&crate::map::TreasureDeposit>,
) {
    debug!(
        "MAP_GEN: Phase changed to {:?}. Checking for auto-fill...",
        *phase.get()
    );

    let current_phase = *phase.get();
    let needs_auto_fill = match current_phase {
        EditorPhase::Landscape => !map_data
            .tiles
            .values()
            .any(|t| t.landscape_feature != LandscapeFeature::None),
        EditorPhase::Sediments => !map_data
            .tiles
            .values()
            .any(|t| t.terrain != TerrainType::Grass || t.forest_type != ForestType::None),
        EditorPhase::NPCs => q_pois.is_empty() && q_camps.is_empty(),
        EditorPhase::Plants => q_deposits.is_empty(),
        EditorPhase::Treasures => q_treasures.is_empty(),
        _ => false,
    };

    ev_gen.write(GenerateMapEvent {
        mode: GenerationMode::Preserve,
        auto_fill_phase: if needs_auto_fill {
            Some(current_phase)
        } else {
            None
        },
    });
}
