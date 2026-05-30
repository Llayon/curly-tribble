use crate::game_state::{ArtifactToolState, EditorPhase};
use crate::map::{Artifact, ArtifactLocation, HexCoord, ResourceType, TargetEntity, TradeConfig};
use bevy::prelude::*;
use bevy_egui::egui;

pub struct ArtifactInspectorPlugin;

impl Plugin for ArtifactInspectorPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn show_artifact_hierarchy(
    ui: &mut egui::Ui,
    current_phase: &EditorPhase,
    artifact_state: &mut ArtifactToolState,
    q_artifacts: &Query<(TargetEntity, &Artifact)>,
) {
    if *current_phase == EditorPhase::Artifacts {
        ui.collapsing("🏺 Artifacts", |ui| {
            let mut to_select = None;
            for (entity, artifact) in q_artifacts.iter() {
                let is_selected = artifact_state.selected_artifact == Some(entity);
                if ui
                    .selectable_label(is_selected, format!("{:?}", artifact.artifact_type))
                    .clicked()
                {
                    to_select = Some(entity);
                }
            }
            if let Some(ent) = to_select {
                artifact_state.selected_artifact = Some(ent);
            }
        });
    }
}

pub fn show_artifact_properties(
    ui: &mut egui::Ui,
    art_ent: TargetEntity,
    artifact_state: &mut ArtifactToolState,
    q_artifacts: &mut Query<(TargetEntity, &mut Artifact), With<Artifact>>,
) {
    if let Ok((_, mut artifact)) = q_artifacts.get_mut(art_ent) {
        ui.label(format!("Artifact: {:?}", artifact.artifact_type));

        let current_loc_str = match artifact.location {
            ArtifactLocation::InTreasure(_) => "InTreasure",
            ArtifactLocation::OnGround(_) => "OnGround",
            ArtifactLocation::InTrade(_) => "InTrade",
        };

        let mut new_loc_str = current_loc_str;
        egui::ComboBox::from_id_salt("artifact_location_prop")
            .selected_text(current_loc_str)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut new_loc_str, "InTreasure", "InTreasure");
                ui.selectable_value(&mut new_loc_str, "OnGround", "OnGround");
                ui.selectable_value(&mut new_loc_str, "InTrade", "InTrade");
            });

        if new_loc_str != current_loc_str {
            match new_loc_str {
                "InTreasure" => {
                    artifact.location = ArtifactLocation::InTreasure(Entity::PLACEHOLDER)
                }
                "OnGround" => {
                    artifact.location = ArtifactLocation::OnGround(HexCoord::new(0, 0));
                    artifact_state.placing_on_ground = true;
                }
                "InTrade" => {
                    artifact.location = ArtifactLocation::InTrade(TradeConfig {
                        faction_id: 0,
                        cost_type: ResourceType::Gold,
                        cost_amount: 100,
                        unlock_condition: String::new(),
                    })
                }
                _ => {}
            }
        }

        if let ArtifactLocation::InTrade(trade_config) = &mut artifact.location {
            ui.label("Trade Config:");
            ui.horizontal(|ui| {
                ui.label("Faction ID:");
                ui.add(egui::DragValue::new(&mut trade_config.faction_id));
            });
            ui.horizontal(|ui| {
                ui.label("Cost Amount:");
                ui.add(egui::DragValue::new(&mut trade_config.cost_amount));
            });
            ui.horizontal(|ui| {
                ui.label("Unlock Condition:");
                ui.text_edit_singleline(&mut trade_config.unlock_condition);
            });
        }
    }
}
