use crate::game_state::EditorPhase;
use crate::map::RebuildMeshEvent;
use bevy::prelude::*;
use bevy_egui::egui;

pub struct TopBarPlugin;

impl Plugin for TopBarPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn show_top_bar(
    ctx: &egui::Context,
    terrain_config: &mut crate::map::terrain_gen::TerrainConfig,
    current_phase: &EditorPhase,
    ev_rebuild: &mut MessageWriter<RebuildMeshEvent>,
) {
    egui::TopBottomPanel::top("top_filter_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("VIEW:");
            if ui
                .selectable_label(terrain_config.show_forests, "🌲 Forests")
                .clicked()
            {
                terrain_config.show_forests = !terrain_config.show_forests;
                ev_rebuild.write(RebuildMeshEvent);
            }
            if ui
                .selectable_label(terrain_config.show_factions, "🚩 Factions")
                .clicked()
            {
                terrain_config.show_factions = !terrain_config.show_factions;
                ev_rebuild.write(RebuildMeshEvent);
            }
            if ui
                .selectable_label(terrain_config.show_cliffs, "📐 Cliffs")
                .clicked()
            {
                terrain_config.show_cliffs = !terrain_config.show_cliffs;
            }
            if ui
                .selectable_label(terrain_config.show_build_area, "🧱 Build Area")
                .clicked()
            {
                terrain_config.show_build_area = !terrain_config.show_build_area;
                ev_rebuild.write(RebuildMeshEvent);
            }
            ui.separator();
            ui.label(format!("Phase: {:?}", current_phase));
        });
    });
}
