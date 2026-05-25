use crate::game_state::{CurrentTool, EditorPhase, LandscapeTool, NpcTool, ShapeTool};
use crate::map::{DepositType, ForestType, TerrainType};
use bevy::prelude::*;
use bevy_egui::egui;

pub fn show_tools_sidebar(
    ctx: &egui::Context,
    current_phase: &EditorPhase,
    current_tool: &mut ResMut<CurrentTool>,
) {
    egui::SidePanel::left("tool_sidebar")
        .default_width(120.0)
        .show(ctx, |ui| {
            ui.heading("Tools");
            ui.separator();

            match current_phase {
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
                EditorPhase::Sediments => {
                    ui.label("Sediment Tool:");
                    ui.checkbox(&mut current_tool.active_sediment_tool, "Active");
                    egui::ComboBox::from_id_salt("sediment_type")
                        .selected_text(format!("{:?}", current_tool.sediment))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Dirt,
                                "Dirt",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Dusty,
                                "Dusty",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Fertile,
                                "Fertile",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Mossy,
                                "Mossy",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Steppe,
                                "Steppe",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Stony,
                                "Stony",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Swamp,
                                "Swamp",
                            );
                        });

                    ui.separator();
                    ui.label("Forest Tool:");
                    ui.checkbox(&mut current_tool.active_forest_tool, "Active");
                    egui::ComboBox::from_id_salt("forest_type")
                        .selected_text(format!("{:?}", current_tool.forest_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut current_tool.forest_type,
                                ForestType::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut current_tool.forest_type,
                                ForestType::Deciduous,
                                "Deciduous",
                            );
                            ui.selectable_value(
                                &mut current_tool.forest_type,
                                ForestType::Coniferous,
                                "Coniferous",
                            );
                        });
                    ui.add(
                        egui::Slider::new(&mut current_tool.forest_density, 0.0..=1.0)
                            .text("Density"),
                    );
                }
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
                EditorPhase::Plants => {
                    ui.label("Bio-Deposit Tools:");
                    egui::ComboBox::from_id_salt("bio_resource_type")
                        .selected_text(format!("{:?}", current_tool.bio_resource))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut current_tool.bio_resource,
                                DepositType::Rabbit,
                                "Rabbit",
                            );
                            ui.selectable_value(
                                &mut current_tool.bio_resource,
                                DepositType::Deer,
                                "Deer",
                            );
                            ui.selectable_value(
                                &mut current_tool.bio_resource,
                                DepositType::Boar,
                                "Boar",
                            );
                            ui.selectable_value(
                                &mut current_tool.bio_resource,
                                DepositType::WildFlax,
                                "WildFlax",
                            );
                            ui.selectable_value(
                                &mut current_tool.bio_resource,
                                DepositType::Raspberries,
                                "Raspberries",
                            );
                            ui.selectable_value(
                                &mut current_tool.bio_resource,
                                DepositType::Pumpkin,
                                "Pumpkin",
                            );
                            ui.selectable_value(
                                &mut current_tool.bio_resource,
                                DepositType::WildWheat,
                                "WildWheat",
                            );
                            ui.selectable_value(
                                &mut current_tool.bio_resource,
                                DepositType::OceanFish,
                                "OceanFish",
                            );
                        });
                    ui.add(egui::Slider::new(&mut current_tool.bio_amount, 1..=100).text("Amount"));
                    ui.add(
                        egui::Slider::new(&mut current_tool.bio_brush_size, 1..=5)
                            .text("Brush Size"),
                    );
                }
                _ => {
                    ui.label("No tools for this phase.");
                }
            }
        });
}
