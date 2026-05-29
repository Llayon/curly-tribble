use crate::game_state::EditorPhase;
use crate::map::GenerateMapEvent;
use crate::map::{ForestType, LandscapeFeature, MapData, TerrainType};
use bevy::prelude::*;

pub struct PhaseTransitionsPlugin;

impl Plugin for PhaseTransitionsPlugin {
    fn build(&self, _app: &mut App) {}
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
        force_reset: false,
        auto_fill_phase: if needs_auto_fill {
            Some(current_phase)
        } else {
            None
        },
    });
}
