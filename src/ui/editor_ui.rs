use crate::game_state::{CurrentTool, EditorPhase, GameState};
use crate::map::MapData;
use bevy::prelude::*;
use bevy_egui::EguiContexts;

use super::panels;

#[derive(Resource, Debug, Default, Reflect)]
pub struct UiExecutionTracker {
    pub primary_context_count: usize,
    pub main_camera_has_primary_context: u8,
    pub game_state_editing_reached: u8,
    pub editor_ui_executed: u8,
    pub top_bar_executed: u8,
    pub bottom_bar_executed: u8,
    pub tools_sidebar_executed: u8,
    pub inspector_sidebar_executed: u8,
}

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiExecutionTracker>()
            .register_type::<UiExecutionTracker>();

        app.add_systems(
            bevy_egui::EguiPrimaryContextPass,
            editor_phase_ui.run_if(in_state(GameState::Editing)),
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
    (mut tracker, q_primary_contexts, q_main_camera): (
        ResMut<UiExecutionTracker>,
        Query<(), With<bevy_egui::PrimaryEguiContext>>,
        Query<Entity, (With<Camera3d>, With<bevy_egui::PrimaryEguiContext>)>,
    ),
    mut logged_error: Local<bool>,
) {
    let primary_count = q_primary_contexts.iter().count();
    tracker.primary_context_count = primary_count;
    tracker.main_camera_has_primary_context = u8::from(!q_main_camera.is_empty());
    tracker.game_state_editing_reached = 1;

    let ctx = match q_main_camera.single() {
        Ok(cam_entity) => match contexts.ctx_for_entity_mut(cam_entity) {
            Ok(c) => c,
            Err(err) => {
                if !*logged_error {
                    *logged_error = true;
                    error!(
                        "editor_phase_ui failed to acquire egui context for MainCamera entity ({:?}): {:?}. Cameras with PrimaryEguiContext count: {}",
                        cam_entity, err, primary_count
                    );
                }
                return;
            }
        },
        Err(err) => match contexts.ctx_mut() {
            Ok(c) => c,
            Err(ctx_err) => {
                if !*logged_error {
                    *logged_error = true;
                    error!(
                        "editor_phase_ui failed to acquire primary egui context (SingleCam err: {:?}, ctx_mut err: {:?}). Cameras with PrimaryEguiContext count: {}",
                        err, ctx_err, primary_count
                    );
                }
                return;
            }
        },
    };

    ctx.set_visuals(bevy_egui::egui::Visuals::dark());
    tracker.editor_ui_executed = 1;

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
    tracker.top_bar_executed = 1;

    panels::bottom_bar::show_bottom_bar(
        ctx,
        current_phase.get(),
        &mut next_phase,
        validation_state,
    );
    tracker.bottom_bar_executed = 1;

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
    tracker.tools_sidebar_executed = 1;

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
    tracker.inspector_sidebar_executed = 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_egui_primary_context_and_panels_execution() {
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

        app.world_mut().spawn(crate::camera::MainCameraBundle {
            camera_3d: Camera3d::default(),
            ui_camera: bevy::ui::IsDefaultUiCamera,
            egui_context: bevy_egui::PrimaryEguiContext,
            transform: Transform::from_xyz(0.0, 30.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
            focus: crate::camera::CameraFocus(Vec3::ZERO),
            config: crate::camera::CameraConfig::default(),
            name: Name::new("Main Camera"),
        });

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Editing);

        app.finish();
        app.cleanup();
        app.update();

        // Check 1: Main Camera has PrimaryEguiContext
        let mut main_camera_query = app
            .world_mut()
            .query_filtered::<Entity, (With<Camera3d>, With<bevy_egui::PrimaryEguiContext>)>();
        let main_cam_count = main_camera_query.iter(app.world()).count();
        assert_eq!(
            main_cam_count, 1,
            "Main Camera must have PrimaryEguiContext attached"
        );

        // Check 2: Exactly one PrimaryEguiContext exists in the application
        let mut all_primary_ctx_query = app
            .world_mut()
            .query_filtered::<Entity, With<bevy_egui::PrimaryEguiContext>>();
        let total_primary_count = all_primary_ctx_query.iter(app.world()).count();
        assert_eq!(
            total_primary_count, 1,
            "Exactly one PrimaryEguiContext must exist in app"
        );

        // Check 3: GameState reaches Editing
        let state = app.world().resource::<State<GameState>>();
        assert_eq!(
            *state.get(),
            GameState::Editing,
            "GameState must reach Editing"
        );

        // Check 4 & 5: Tracker confirms editor_phase_ui and all four panels executed
        let tracker = app.world().resource::<UiExecutionTracker>();
        assert_eq!(
            tracker.main_camera_has_primary_context, 1,
            "Tracker confirms Main Camera has PrimaryEguiContext"
        );
        assert_eq!(
            tracker.primary_context_count, 1,
            "Tracker confirms primary_context_count == 1"
        );
        assert_eq!(
            tracker.game_state_editing_reached, 1,
            "Tracker confirms GameState::Editing reached"
        );
        assert_eq!(
            tracker.editor_ui_executed, 1,
            "Tracker confirms editor_phase_ui executed without NoEntities error"
        );
        assert_eq!(
            tracker.top_bar_executed, 1,
            "Tracker confirms top_bar executed"
        );
        assert_eq!(
            tracker.bottom_bar_executed, 1,
            "Tracker confirms bottom_bar executed"
        );
        assert_eq!(
            tracker.tools_sidebar_executed, 1,
            "Tracker confirms tools_sidebar executed"
        );
        assert_eq!(
            tracker.inspector_sidebar_executed, 1,
            "Tracker confirms inspector_sidebar executed"
        );
    }
}
