use crate::game_state::{CurrentTool, EditorPhase, GameState};
use crate::map::MapData;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use super::panels;

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            bevy_egui::EguiPrimaryContextPass,
            editor_phase_ui
                .map(drop)
                .run_if(in_state(GameState::Editing)),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn editor_phase_ui(
    mut contexts: EguiContexts,
    current_phase: Res<State<EditorPhase>>,
    mut next_phase: ResMut<NextState<EditorPhase>>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut current_tool: ResMut<CurrentTool>,
    mut link_state: ResMut<crate::map::LinkToolState>,
    mut faction_manager: ResMut<crate::game_state::FactionManager>,
    map_data: Res<MapData>,
    mut terrain_config: ResMut<crate::map::terrain_gen::TerrainConfig>,
    mut ev_rebuild: MessageWriter<crate::map::RebuildMeshEvent>,
    (q_selected_treasures, mut artifact_state, mut q_artifacts): (
        Query<
            (
                crate::map::TargetEntity,
                &mut crate::map::treasures::TreasureDeposit,
            ),
            (
                With<crate::game_state::Selected>,
                With<crate::map::treasures::TreasureDeposit>,
            ),
        >,
        ResMut<crate::game_state::ArtifactToolState>,
        Query<(crate::map::TargetEntity, &mut crate::map::Artifact), With<crate::map::Artifact>>,
    ),
    mut commands: Commands,
    (q_mines, mut q_selected_mines): (
        Query<(Entity, &crate::map::mines::MineDeposit), Without<crate::game_state::Selected>>,
        Query<
            (Entity, &mut crate::map::mines::MineDeposit),
            (
                With<crate::game_state::Selected>,
                With<crate::map::mines::MineDeposit>,
            ),
        >,
    ),
    q_starter_resources: (
        Query<&crate::map::ResourceDeposit>,
        Query<&Transform, With<crate::map::resources::BerryBush>>,
    ),
) -> Result<(), String> {
    let ctx = contexts.ctx_mut().map_err(|e| e.to_string())?;
    ctx.set_visuals(bevy_egui::egui::Visuals::dark());

    let (q_deposits, q_bushes) = q_starter_resources;
    let is_valid = map_data.validation_errors.is_empty();
    let validation_state = if is_valid {
        panels::bottom_bar::MapValidationState::Valid
    } else {
        panels::bottom_bar::MapValidationState::Invalid
    };

    // Dispatch to modular panels
    panels::top_bar::show_top_bar(
        ctx,
        &mut terrain_config,
        current_phase.get(),
        &mut ev_rebuild,
    );

    panels::bottom_bar::show_bottom_bar(
        ctx,
        current_phase.get(),
        &mut next_phase,
        validation_state,
    );

    panels::tools::show_tools_sidebar(
        ctx,
        current_phase.get(),
        &mut current_tool,
        &mut link_state,
        &map_data,
        &q_deposits,
        &q_bushes,
        &mut commands,
        &mut next_game_state,
    );

    panels::inspector::show_inspector_sidebar(
        ctx,
        current_phase.get(),
        &map_data,
        &mut faction_manager,
        &mut current_tool,
        q_selected_treasures,
        validation_state,
        &mut artifact_state,
        &mut q_artifacts,
        &mut commands,
        &q_mines,
        &mut q_selected_mines,
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_egui_primary_context_single_camera() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::state::app::StatesPlugin,
            bevy::asset::AssetPlugin::default(),
            bevy::input::InputPlugin,
            bevy::window::WindowPlugin {
                primary_window: None,
                exit_condition: bevy::window::ExitCondition::DontExit,
                ..default()
            },
            bevy::gizmos::GizmoPlugin,
        ));
        app.init_asset::<Shader>();
        app.init_asset::<Mesh>();
        app.init_asset::<StandardMaterial>();
        app.init_asset::<Scene>();
        app.init_asset::<Image>();
        app.add_message::<bevy::window::CursorMoved>();
        app.add_message::<bevy::window::FileDragAndDrop>();
        app.add_message::<bevy::input::mouse::MouseWheel>();
        app.add_message::<bevy::input::mouse::MouseButtonInput>();
        app.add_message::<bevy::input::keyboard::KeyboardInput>();
        app.init_resource::<bevy::gizmos::config::GizmoConfigStore>();
        app.add_plugins((
            bevy_egui::EguiPlugin::default(),
            crate::sets::SetsPlugin,
            crate::events::EventsPlugin,
            crate::game_state::GameStatePlugin,
            crate::economy::EconomyPlugin,
            crate::camera::CameraPlugin,
            crate::map::MapPlugin,
            super::EditorUiPlugin,
        ));

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Editing);

        app.finish();
        app.cleanup();
        app.update();

        // 1. Main Camera spawned by CameraPlugin has PrimaryEguiContext attached
        let mut main_camera_query = app
            .world_mut()
            .query_filtered::<Entity, (With<Camera3d>, With<bevy_egui::PrimaryEguiContext>)>();
        let main_cam_count = main_camera_query.iter(app.world()).count();
        assert_eq!(
            main_cam_count, 1,
            "Main Camera must have PrimaryEguiContext attached"
        );

        // 2. Exactly one PrimaryEguiContext exists in the application
        let mut all_primary_ctx_query = app
            .world_mut()
            .query_filtered::<Entity, With<bevy_egui::PrimaryEguiContext>>();
        let total_primary_count = all_primary_ctx_query.iter(app.world()).count();
        assert_eq!(
            total_primary_count, 1,
            "Exactly one PrimaryEguiContext must exist in app"
        );

        // 3. GameState reaches Editing
        let state = app.world().resource::<State<GameState>>();
        assert_eq!(
            *state.get(),
            GameState::Editing,
            "GameState must reach Editing"
        );
    }
}
