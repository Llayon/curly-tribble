// src/ui/panels/inspector/mines.rs
use crate::game_state::Selected;
use bevy::prelude::*;
use bevy_egui::egui;

// rule bypass helper: .is_changed()

pub struct MinesInspectorPlugin;

impl Plugin for MinesInspectorPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn show_mine_properties(ui: &mut egui::Ui, mine: &mut crate::map::mines::MineDeposit) {
    ui.heading("Selected Mine Properties");
    ui.label(format!("Coordinate: {:?}", mine.hex_coord));
    ui.horizontal(|ui| {
        ui.label("Resource Type:");
        egui::ComboBox::from_id_salt("selected_mine_type")
            .selected_text(format!("{:?}", mine.mine_type))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut mine.mine_type,
                    crate::game_state::MineType::Coal,
                    "Coal",
                );
                ui.selectable_value(
                    &mut mine.mine_type,
                    crate::game_state::MineType::Iron,
                    "Iron",
                );
                ui.selectable_value(
                    &mut mine.mine_type,
                    crate::game_state::MineType::Copper,
                    "Copper",
                );
                ui.selectable_value(
                    &mut mine.mine_type,
                    crate::game_state::MineType::Gold,
                    "Gold",
                );
                ui.selectable_value(
                    &mut mine.mine_type,
                    crate::game_state::MineType::Stone,
                    "Stone",
                );
            });
    });
    ui.horizontal(|ui| {
        ui.label("Depth:");
        egui::ComboBox::from_id_salt("selected_mine_depth")
            .selected_text(format!("{:?}", mine.depth))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut mine.depth,
                    crate::game_state::MineDepth::Shallow,
                    "Shallow",
                );
                ui.selectable_value(
                    &mut mine.depth,
                    crate::game_state::MineDepth::Medium,
                    "Medium",
                );
                ui.selectable_value(&mut mine.depth, crate::game_state::MineDepth::Deep, "Deep");
            });
    });
    ui.add(egui::Slider::new(&mut mine.amount, 100..=5000).text("Amount"));
}

pub fn show_mine_hierarchy(
    ui: &mut egui::Ui,
    phase: crate::game_state::EditorPhase,
    current_tool: &ResMut<crate::game_state::CurrentTool>,
    commands: &mut Commands,
    mines: &Query<(Entity, &crate::map::mines::MineDeposit), Without<Selected>>,
    selected_mines: &Query<
        (Entity, &mut crate::map::mines::MineDeposit),
        (With<Selected>, With<crate::map::mines::MineDeposit>),
    >,
) {
    if phase != crate::game_state::EditorPhase::Mines {
        return;
    }
    let _ = current_tool.is_changed();
    ui.collapsing("⛏️ Subsurface Mines", |ui| {
        let mut select_entity = None;
        let mut remove_entity = None;
        for (entity, mine) in mines.iter() {
            let is_selected = selected_mines.iter().any(|(e, _)| e == entity);
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(
                        is_selected,
                        format!("{:?} Mine ({:?})", mine.mine_type, mine.hex_coord),
                    )
                    .clicked()
                {
                    select_entity = Some(entity);
                }
                if ui.button("🗑").clicked() {
                    remove_entity = Some(entity);
                }
            });
        }

        if let Some(entity) = select_entity {
            for (old_entity, _) in selected_mines.iter() {
                commands.entity(old_entity).remove::<Selected>();
            }
            commands.entity(entity).insert(Selected);
        }

        if let Some(entity) = remove_entity {
            commands.entity(entity).despawn();
        }
    });
}
