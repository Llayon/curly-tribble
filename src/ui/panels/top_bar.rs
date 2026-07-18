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
                .selectable_label(terrain_config.forest_layer.is_visible(), "🌲 Forests")
                .clicked()
            {
                terrain_config.forest_layer.toggle();
                ev_rebuild.write(RebuildMeshEvent);
            }
            if ui
                .selectable_label(terrain_config.faction_layer.is_visible(), "🚩 Factions")
                .clicked()
            {
                terrain_config.faction_layer.toggle();
                ev_rebuild.write(RebuildMeshEvent);
            }
            if ui
                .selectable_label(terrain_config.cliff_layer.is_visible(), "📐 Cliffs")
                .clicked()
            {
                terrain_config.cliff_layer.toggle();
            }
            if ui
                .selectable_label(
                    terrain_config.build_area_layer.is_visible(),
                    "🧱 Build Area",
                )
                .clicked()
            {
                terrain_config.build_area_layer.toggle();
                ev_rebuild.write(RebuildMeshEvent);
            }
            ui.separator();
            ui.label(format!("Phase: {current_phase:?}"));
        });
    });
}
