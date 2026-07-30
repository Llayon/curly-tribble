// src/ui/panels/tools_sub.rs
use crate::game_state::{CurrentTool, TreasureToolMode};
use crate::map::deposits::DepositType;
use crate::map::{ForestType, LinkToolState, TerrainType};
use bevy::prelude::*;
use bevy_egui::egui;

pub struct ToolsSubPlugin;

impl Plugin for ToolsSubPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn show_sediment_tools(ui: &mut egui::Ui, tool: &mut CurrentTool) {
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

pub fn show_treasure_tools(
    ui: &mut egui::Ui,
    tool: &mut CurrentTool,
    link_state: &mut LinkToolState,
) {
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

pub fn show_bio_tools(ui: &mut egui::Ui, tool: &mut CurrentTool) {
    ui.label("Bio-Deposit Tools:");
    egui::ComboBox::from_id_salt("bio_resource_type")
        .selected_text(format!("{:?}", tool.bio_resource))
        .show_ui(ui, |ui| {
            for (d, l) in [
                (DepositType::Rabbit, "Rabbit"),
                (DepositType::Deer, "Deer"),
                (DepositType::Boar, "Boar"),
                (DepositType::WildFlax, "WildFlax"),
                (DepositType::Raspberries, "Raspberries"),
                (DepositType::Pumpkin, "Pumpkin"),
                (DepositType::WildWheat, "WildWheat"),
                (DepositType::OceanFish, "OceanFish"),
            ] {
                ui.selectable_value(&mut tool.bio_resource, d, l);
            }
        });
    ui.add(egui::Slider::new(&mut tool.bio_amount, 1..=100).text("Amount"));
    ui.add(egui::Slider::new(&mut tool.bio_brush_size, 1..=5).text("Brush Size"));
}

pub fn show_mine_tools(ui: &mut egui::Ui, tool: &mut CurrentTool) {
    ui.label("Mine Tools:");
    egui::ComboBox::from_id_salt("mine_type_selector")
        .selected_text(format!("{:?}", tool.mine_type))
        .show_ui(ui, |ui| {
            for (mt, l) in [
                (crate::game_state::MineType::Coal, "Coal"),
                (crate::game_state::MineType::Iron, "Iron"),
                (crate::game_state::MineType::Copper, "Copper"),
                (crate::game_state::MineType::Gold, "Gold"),
                (crate::game_state::MineType::Stone, "Stone"),
            ] {
                ui.selectable_value(&mut tool.mine_type, mt, l);
            }
        });

    egui::ComboBox::from_id_salt("mine_depth_selector")
        .selected_text(format!("{:?}", tool.mine_depth))
        .show_ui(ui, |ui| {
            for (md, l) in [
                (crate::game_state::MineDepth::Shallow, "Shallow"),
                (crate::game_state::MineDepth::Medium, "Medium"),
                (crate::game_state::MineDepth::Deep, "Deep"),
            ] {
                ui.selectable_value(&mut tool.mine_depth, md, l);
            }
        });

    ui.add(egui::Slider::new(&mut tool.mine_amount, 100..=5000).text("Amount"));
    ui.add(egui::Slider::new(&mut tool.mine_brush_size, 1..=5).text("Brush Size"));

    ui.separator();
    ui.label("Tool Mode:");
    for (mode, label) in [
        (crate::game_state::MineTool::Paint, "Paint"),
        (crate::game_state::MineTool::Delete, "Delete"),
    ] {
        if ui.selectable_label(tool.mine_tool == mode, label).clicked() {
            tool.mine_tool = mode;
        }
    }
}
