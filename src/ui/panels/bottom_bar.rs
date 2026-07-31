use crate::game_state::EditorPhase;
use bevy::prelude::*;
use bevy_egui::egui;

pub struct BottomBarPlugin;

impl Plugin for BottomBarPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(PartialEq, Clone, Copy)]
pub enum MapValidationState {
    Valid,
    Invalid,
}

pub fn show_bottom_bar(
    ctx: &egui::Context,
    current_phase: &EditorPhase,
    next_phase: &mut ResMut<NextState<EditorPhase>>,
    validation_state: MapValidationState,
) {
    let is_valid = validation_state == MapValidationState::Valid;
    let panel_frame = egui::Frame::NONE
        .fill(egui::Color32::from_rgb(20, 22, 30))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(60, 65, 80)));
    egui::TopBottomPanel::bottom("phase_timeline")
        .frame(panel_frame)
        .show(ctx, |ui| {
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let phases = [
                        (EditorPhase::Shape, "1. Shape"),
                        (EditorPhase::Factions, "2. Factions"),
                        (EditorPhase::Landscape, "3. Landscape"),
                        (EditorPhase::Sediments, "4. Sediments"),
                        (EditorPhase::NPCs, "5. NPCs"),
                        (EditorPhase::Plants, "6. Plants"),
                        (EditorPhase::Treasures, "7. Treasures"),
                        (EditorPhase::Artifacts, "8. Artifacts"),
                        (EditorPhase::Mines, "9. Mines"),
                        (EditorPhase::Balance, "10. Balance"),
                        (EditorPhase::Height3D, "11. Height"),
                        (EditorPhase::Finetuning, "12. Finetuning"),
                        (EditorPhase::Deposits, "13. Deposits"),
                        (EditorPhase::Buildings, "14. Buildings"),
                        (EditorPhase::Villages, "15. Villages"),
                        (EditorPhase::Props, "16. Props"),
                        (EditorPhase::Export, "17. Export"),
                    ];

                    let current_idx = phases
                        .iter()
                        .position(|(p, _)| p == current_phase)
                        .unwrap_or(0);

                    for (idx, (phase, label)) in phases.into_iter().enumerate() {
                        let is_current = *current_phase == phase;
                        let is_physically_reachable = idx <= current_idx + 1;
                        let needs_validation = idx > 1;
                        let can_click = is_physically_reachable && (!needs_validation || is_valid);

                        ui.add_enabled_ui(can_click, |ui| {
                            if ui.selectable_label(is_current, label).clicked() {
                                next_phase.set(phase);
                            }
                        });
                        if idx < phases.len() - 1 {
                            ui.label("→");
                        }
                    }
                });
            });
        });
}
