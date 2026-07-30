use super::tools_sub::{show_bio_tools, show_mine_tools, show_sediment_tools, show_treasure_tools};
use crate::game_state::{CurrentTool, EditorPhase, LandscapeTool, NpcTool, ShapeTool};
use crate::map::LinkToolState;
use bevy::prelude::*;
use bevy_egui::egui;

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[allow(clippy::too_many_lines)]
pub fn show_tools_sidebar(
    ctx: &egui::Context,
    current_phase: &EditorPhase,
    current_tool: &mut ResMut<CurrentTool>,
    link_state: &mut ResMut<LinkToolState>,
    map_data: &crate::map::MapData,
    q_deposits: &Query<&crate::map::ResourceDeposit>,
    q_bushes: &Query<&Transform, With<crate::map::resources::BerryBush>>,
    commands: &mut Commands,
    next_game_state: &mut ResMut<NextState<crate::game_state::GameState>>,
) {
    egui::SidePanel::left("tool_sidebar")
        .default_width(120.0)
        .show(ctx, |ui| {
            ui.heading("Tools");
            ui.separator();

            match current_phase {
                EditorPhase::Treasures => show_treasure_tools(ui, current_tool, link_state),
                EditorPhase::Shape => {
                    ui.label("Island Shape:");
                    if ui
                        .selectable_label(current_tool.shape == ShapeTool::None, "None")
                        .clicked()
                    {
                        current_tool.shape = ShapeTool::None;
                    }
                    if ui
                        .selectable_label(current_tool.shape == ShapeTool::Ocean, "Ocean")
                        .clicked()
                    {
                        current_tool.shape = ShapeTool::Ocean;
                    }
                }
                EditorPhase::Factions => {
                    ui.label("Faction Painting:");
                    if ui
                        .selectable_label(
                            current_tool.faction == crate::game_state::FactionTool::None,
                            "None",
                        )
                        .clicked()
                    {
                        current_tool.faction = crate::game_state::FactionTool::None;
                    }
                    if ui
                        .selectable_label(
                            current_tool.faction == crate::game_state::FactionTool::Brush,
                            "Brush",
                        )
                        .clicked()
                    {
                        current_tool.faction = crate::game_state::FactionTool::Brush;
                    }
                }
                EditorPhase::Landscape => {
                    ui.label("Landscape Brushes:");
                    let tools = [
                        (LandscapeTool::None, "None"),
                        (LandscapeTool::Mountain, "Mountain"),
                        (LandscapeTool::Lake, "Lake"),
                        (LandscapeTool::River, "River"),
                        (LandscapeTool::Plateau, "Plateau"),
                        (LandscapeTool::Cliff, "Cliff"),
                    ];
                    for (tool, label) in tools {
                        if ui
                            .selectable_label(current_tool.landscape == tool, label)
                            .clicked()
                        {
                            current_tool.landscape = tool;
                        }
                    }
                }
                EditorPhase::Sediments => show_sediment_tools(ui, current_tool),
                EditorPhase::NPCs => {
                    ui.label("NPC Tools:");
                    let tools = [
                        (NpcTool::None, "None"),
                        (NpcTool::SpawnPoi, "Spawn POI"),
                        (NpcTool::SpawnEnemyCamp, "Spawn Enemy Camp"),
                        (NpcTool::Delete, "Delete"),
                    ];
                    for (tool, label) in tools {
                        if ui
                            .selectable_label(current_tool.npc == tool, label)
                            .clicked()
                        {
                            current_tool.npc = tool;
                        }
                    }
                }
                EditorPhase::Plants => show_bio_tools(ui, current_tool),
                EditorPhase::Mines => show_mine_tools(ui, current_tool),
                EditorPhase::Balance => {
                    ui.label("Starter Resources:");
                    let deficiencies = crate::map::balance::get_starter_deficiencies(
                        map_data, q_deposits, q_bushes,
                    );

                    let wood_ok =
                        !deficiencies.contains(&crate::map::balance::StarterResource::Wood);
                    let food_ok =
                        !deficiencies.contains(&crate::map::balance::StarterResource::Food);
                    let flax_ok =
                        !deficiencies.contains(&crate::map::balance::StarterResource::Flax);

                    ui.horizontal(|ui| {
                        ui.label("Wood:");
                        if wood_ok {
                            ui.colored_label(egui::Color32::GREEN, "✅");
                        } else {
                            ui.colored_label(egui::Color32::RED, "❌");
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Food:");
                        if food_ok {
                            ui.colored_label(egui::Color32::GREEN, "✅");
                        } else {
                            ui.colored_label(egui::Color32::RED, "❌");
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Flax:");
                        if flax_ok {
                            ui.colored_label(egui::Color32::GREEN, "✅");
                        } else {
                            ui.colored_label(egui::Color32::RED, "❌");
                        }
                    });

                    ui.separator();

                    if ui.button("🔍 Auto-Balance Starter Area").clicked() {
                        use crate::map::balance_commands::BalanceCommandsExt;
                        commands.auto_balance_starter_area(1);
                    }
                }
                EditorPhase::Deposits => {
                    ui.label("Sub-Hex 1x1m Deposits (Auto-snaps Y).");
                    ui.label("Tip: Alt+Drag duplicates objects.");
                }
                EditorPhase::Buildings => {
                    ui.label("Building Foundations (Flattens terrain).");
                }
                EditorPhase::Villages => {
                    ui.label("Villages (Road paths & Border posts).");
                }
                EditorPhase::Props => {
                    ui.label("Decorative Props (Land/Water snapping).");
                }
                EditorPhase::Height3D => {
                    ui.label("3D Height Adjustment (Sculpt relief & peaks).");
                }
                EditorPhase::Finetuning => {
                    ui.label("Finetuning (Micro-adjusting heights & slopes).");
                }
                EditorPhase::Export => {
                    if map_data.validation_errors.is_empty() {
                        ui.colored_label(egui::Color32::GREEN, "✅ All Checks Passed!");
                    } else {
                        ui.colored_label(
                            egui::Color32::RED,
                            format!("❌ {} Issues:", map_data.validation_errors.len()),
                        );
                        for err in &map_data.validation_errors {
                            ui.label(format!("• {err}"));
                        }
                    }
                    if ui.button("💾 Export Map Package (.json)").clicked() {
                        use crate::map::export::ExportMapExt;
                        commands.export_map_package("assets/maps/custom_map.json");
                    }
                    if ui.button("🎮 Launch Playtest").clicked() {
                        next_game_state.set(crate::game_state::GameState::Playing);
                    }
                }
                EditorPhase::Artifacts => {
                    ui.label("No tools for this phase.");
                }
            }
        });
}
