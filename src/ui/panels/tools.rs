use crate::game_state::{
    CurrentTool, EditorPhase, LandscapeTool, NpcTool, ShapeTool, TreasureToolMode,
};
use crate::map::{DepositType, ForestType, LinkToolState, TerrainType};
use bevy::prelude::*;
use bevy_egui::egui;

pub struct ToolsPlugin;

impl Plugin for ToolsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn show_tools_sidebar(
    ctx: &egui::Context,
    current_phase: &EditorPhase,
    current_tool: &mut ResMut<CurrentTool>,
    link_state: &mut ResMut<LinkToolState>,
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
                _ => {
                    ui.label("No tools for this phase.");
                }
            }
        });
}

fn show_sediment_tools(ui: &mut egui::Ui, tool: &mut CurrentTool) {
    ui.label("Sediment Tool:");
    ui.checkbox(&mut tool.active_sediment_tool, "Active");
    egui::ComboBox::from_id_salt("sediment_type")
        .selected_text(format!("{:?}", tool.sediment))
        .show_ui(ui, |ui| {
            for (terrain, label) in [
                (TerrainType::Dirt, "Dirt"),
                (TerrainType::Dusty, "Dusty"),
                (TerrainType::Fertile, "Fertile"),
                (TerrainType::Mossy, "Mossy"),
                (TerrainType::Steppe, "Steppe"),
                (TerrainType::Stony, "Stony"),
                (TerrainType::Swamp, "Swamp"),
            ] {
                ui.selectable_value(&mut tool.sediment, terrain, label);
            }
        });
    ui.separator();
    ui.label("Forest Tool:");
    ui.checkbox(&mut tool.active_forest_tool, "Active");
    egui::ComboBox::from_id_salt("forest_type")
        .selected_text(format!("{:?}", tool.forest_type))
        .show_ui(ui, |ui| {
            for (forest, label) in [
                (ForestType::None, "None"),
                (ForestType::Deciduous, "Deciduous"),
                (ForestType::Coniferous, "Coniferous"),
            ] {
                ui.selectable_value(&mut tool.forest_type, forest, label);
            }
        });
    ui.add(egui::Slider::new(&mut tool.forest_density, 0.0..=1.0).text("Density"));
}

fn show_treasure_tools(ui: &mut egui::Ui, tool: &mut CurrentTool, link_state: &mut LinkToolState) {
    ui.label("Treasure Tools:");
    for (mode, label) in [
        (TreasureToolMode::SpawnVisible, "Spawn Visible"),
        (TreasureToolMode::SpawnHidden, "Spawn Hidden"),
        (TreasureToolMode::Link, "Link Tool"),
    ] {
        if ui
            .selectable_label(tool.treasure_mode == mode, label)
            .clicked()
        {
            tool.treasure_mode = mode;
        }
    }
    if !matches!(link_state, LinkToolState::Idle) {
        ui.separator();
        ui.colored_label(egui::Color32::YELLOW, "Link Active");
        if ui.button("Reset Link Tool").clicked() {
            *link_state = LinkToolState::Idle;
        }
    }
}

fn show_bio_tools(ui: &mut egui::Ui, tool: &mut CurrentTool) {
    ui.label("Bio-Deposit Tools:");
    egui::ComboBox::from_id_salt("bio_resource_type")
        .selected_text(format!("{:?}", tool.bio_resource))
        .show_ui(ui, |ui| {
            for (deposit, label) in [
                (DepositType::Rabbit, "Rabbit"),
                (DepositType::Deer, "Deer"),
                (DepositType::Boar, "Boar"),
                (DepositType::WildFlax, "WildFlax"),
                (DepositType::Raspberries, "Raspberries"),
                (DepositType::Pumpkin, "Pumpkin"),
                (DepositType::WildWheat, "WildWheat"),
                (DepositType::OceanFish, "OceanFish"),
            ] {
                ui.selectable_value(&mut tool.bio_resource, deposit, label);
            }
        });
    ui.add(egui::Slider::new(&mut tool.bio_amount, 1..=100).text("Amount"));
    ui.add(egui::Slider::new(&mut tool.bio_brush_size, 1..=5).text("Brush Size"));
}
