use crate::game_state::{
    ArtifactToolState, CurrentTool, EditorPhase, Faction, FactionManager, FactionType, NpcTool,
    Selected,
};
use crate::map::{MapData, PoiType, TreasureDeposit};
use bevy::prelude::*;
use bevy_egui::egui;

pub mod artifacts;
pub mod treasures;

pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn show_inspector_sidebar(
    ctx: &egui::Context,
    current_phase: &EditorPhase,
    map_data: &MapData,
    faction_manager: &mut ResMut<FactionManager>,
    current_tool: &mut ResMut<CurrentTool>,
    mut q_selected_treasures: Query<
        (crate::map::TargetEntity, &mut TreasureDeposit),
        (With<Selected>, With<TreasureDeposit>),
    >,
    validation_state: super::bottom_bar::MapValidationState,
    artifact_state: &mut ResMut<ArtifactToolState>,
    q_artifacts: &mut Query<
        (crate::map::TargetEntity, &mut crate::map::Artifact),
        With<crate::map::Artifact>,
    >,
) {
    let is_valid = validation_state == super::bottom_bar::MapValidationState::Valid;
    egui::SidePanel::right("inspector_sidebar")
        .default_width(250.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if !is_valid {
                    ui.collapsing("⚠️ Validation Errors", |ui| {
                        for err in &map_data.validation_errors {
                            ui.colored_label(egui::Color32::RED, format!("• {err}"));
                        }
                    });
                }

                // Selected Treasure Properties
                if let Ok((_entity, mut deposit)) = q_selected_treasures.single_mut() {
                    treasures::show_treasure_properties(ui, &mut deposit);
                }

                show_faction_hierarchy(ui, *current_phase, faction_manager);

                show_artifact_hierarchy(ui, *current_phase, artifact_state, q_artifacts);

                show_selection_properties(
                    ui,
                    faction_manager,
                    current_tool,
                    artifact_state,
                    q_artifacts,
                );
            });
        });
}

fn show_artifact_hierarchy(
    ui: &mut egui::Ui,
    phase: EditorPhase,
    artifact_state: &mut ArtifactToolState,
    artifacts: &Query<
        (crate::map::TargetEntity, &mut crate::map::Artifact),
        With<crate::map::Artifact>,
    >,
) {
    if phase != EditorPhase::Artifacts {
        return;
    }
    ui.collapsing("🏺 Artifacts", |ui| {
        let mut selected = None;
        for (entity, artifact) in artifacts.iter() {
            if ui
                .selectable_label(
                    artifact_state.selected_artifact == Some(entity),
                    format!("{:?}", artifact.artifact_type),
                )
                .clicked()
            {
                selected = Some(entity);
            }
        }
        if let Some(entity) = selected {
            artifact_state.selected_artifact = Some(entity);
        }
    });
}

fn show_selection_properties(
    ui: &mut egui::Ui,
    factions: &mut FactionManager,
    tool: &mut CurrentTool,
    artifact_state: &mut ArtifactToolState,
    artifacts: &mut Query<
        (crate::map::TargetEntity, &mut crate::map::Artifact),
        With<crate::map::Artifact>,
    >,
) {
    ui.collapsing("🔍 Selection Properties", |ui| {
        if let Some(id) = factions.selected_faction {
            if let Some(faction) = factions.factions.iter_mut().find(|f| f.id == id) {
                ui.label(format!("Editing: {}", faction.name));
                ui.text_edit_singleline(&mut faction.name);
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    egui::ComboBox::from_id_salt("faction_type_prop")
                        .selected_text(format!("{:?}", faction.faction_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut faction.faction_type,
                                FactionType::Player,
                                "Player",
                            );
                            ui.selectable_value(
                                &mut faction.faction_type,
                                FactionType::Neutral,
                                "Neutral",
                            );
                            ui.selectable_value(
                                &mut faction.faction_type,
                                FactionType::Enemy,
                                "Enemy",
                            );
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Economy:");
                    egui::ComboBox::from_id_salt("economy_focus_prop")
                        .selected_text(&faction.economy_focus)
                        .show_ui(ui, |ui| {
                            for focus in ["None", "Mining", "Farming", "Woodcutting"] {
                                ui.selectable_value(
                                    &mut faction.economy_focus,
                                    focus.to_string(),
                                    focus,
                                );
                            }
                        });
                });
            }
        } else if tool.npc == NpcTool::SpawnEnemyCamp {
            ui.label("Enemy Camp Settings:");
            ui.add(egui::Slider::new(&mut tool.camp_difficulty, 0.0..=1.0).text("Difficulty"));
            ui.add(egui::Slider::new(&mut tool.camp_power, 10..=1000).text("Combat Power"));
        } else if tool.npc == NpcTool::SpawnPoi {
            ui.label("POI Settings:");
            egui::ComboBox::from_id_salt("poi_type_prop")
                .selected_text(format!("{:?}", tool.poi_type))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut tool.poi_type, PoiType::TradePost, "TradePost");
                    ui.selectable_value(&mut tool.poi_type, PoiType::Ruins, "Ruins");
                    ui.selectable_value(&mut tool.poi_type, PoiType::Shrine, "Shrine");
                    ui.selectable_value(&mut tool.poi_type, PoiType::Treasure, "Treasure");
                });
        } else if let Some(entity) = artifact_state.selected_artifact {
            artifacts::show_artifact_properties(ui, entity, artifact_state, artifacts);
        } else {
            ui.label("No selection.");
        }
    });
}

fn show_faction_hierarchy(
    ui: &mut egui::Ui,
    phase: EditorPhase,
    faction_manager: &mut FactionManager,
) {
    if !matches!(
        phase,
        EditorPhase::Factions | EditorPhase::NPCs | EditorPhase::Plants
    ) {
        return;
    }
    ui.collapsing("🚩 Faction Hierarchy", |ui| {
        let mut to_remove = None;
        let mut to_select = None;
        for (idx, faction) in faction_manager.factions.iter().enumerate() {
            let is_selected = faction_manager.selected_faction == Some(faction.id);
            ui.horizontal(|ui| {
                if ui.selectable_label(is_selected, &faction.name).clicked() {
                    to_select = Some(faction.id);
                }
                if faction.faction_type != FactionType::Player && ui.button("🗑").clicked() {
                    to_remove = Some(idx);
                }
            });
        }
        if let Some(id) = to_select {
            faction_manager.selected_faction = Some(id);
        }
        if let Some(idx) = to_remove {
            faction_manager.factions.remove(idx);
            faction_manager.selected_faction = None;
        }
        if ui.button("Add Neutral Faction").clicked() {
            let next_id = faction_manager
                .factions
                .iter()
                .map(|f| f.id)
                .max()
                .unwrap_or(0)
                + 1;
            faction_manager.factions.push(Faction {
                id: next_id,
                name: format!("Faction {next_id}"),
                faction_type: FactionType::Neutral,
                color: Color::srgb(rand::random(), rand::random(), rand::random()),
                economy_focus: "None".to_string(),
            });
        }
    });
}
