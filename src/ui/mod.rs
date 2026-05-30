use crate::economy::GlobalResources;
use crate::game_state::{CurrentTool, EditorPhase, GameState};
use crate::map::MapData;
use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};

pub mod details;
pub mod logs;
pub mod panels;
pub mod resources;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            resources::ResourceUiPlugin,
            details::DetailUiPlugin,
            logs::GameLogPlugin,
        ));

        app.add_systems(Startup, setup_ui.in_set(StartupSet::SpawnEntities))
            .add_systems(
                Update,
                (
                    resources::update_resource_ui
                        .run_if(resource_changed::<GlobalResources>)
                        .in_set(GameSet::Visuals),
                    details::update_settler_detail_ui.in_set(GameSet::Visuals),
                ),
            );

        app.add_systems(
            EguiPrimaryContextPass,
            editor_phase_ui.run_if(in_state(GameState::Playing)),
        );
    }
}

fn editor_phase_ui(
    mut contexts: EguiContexts,
    current_phase: Res<State<EditorPhase>>,
    mut next_phase: ResMut<NextState<EditorPhase>>,
    mut current_tool: ResMut<CurrentTool>,
    mut link_state: ResMut<crate::map::LinkToolState>,
    mut faction_manager: ResMut<crate::game_state::FactionManager>,
    map_data: Res<MapData>,
    mut terrain_config: ResMut<crate::map::terrain_gen::TerrainConfig>,
    mut ev_rebuild: MessageWriter<crate::map::RebuildMeshEvent>,
    q_selected_treasures: Query<
        (
            crate::map::TargetEntity,
            &mut crate::map::treasures::TreasureDeposit,
        ),
        (
            With<crate::game_state::Selected>,
            With<crate::map::treasures::TreasureDeposit>,
        ),
    >,
    mut artifact_state: ResMut<crate::game_state::ArtifactToolState>,
    mut q_artifacts: Query<
        (crate::map::TargetEntity, &mut crate::map::Artifact),
        With<crate::map::Artifact>,
    >,
) {
    let ctx = match contexts.ctx_mut().ok() {
        Some(ctx) => ctx,
        None => return,
    };

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
    panels::tools::show_tools_sidebar(ctx, current_phase.get(), &mut current_tool, &mut link_state);
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
    );
}

fn setup_ui(mut commands: Commands) {
    // 1. Top-left: Global Resources
    let mut resources_node = commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
    ));
    resources::setup_resource_ui(&mut resources_node);

    // 2. Bottom-right: Settler Details
    let mut details_node = commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(15.0)),
            min_width: Val::Px(250.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.2, 0.9)),
    ));
    details::setup_detail_ui(&mut details_node);

    // 3. Bottom-left: Game Log
    let mut log_node = commands.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(10.0),
        left: Val::Px(10.0),
        ..default()
    });
    logs::setup_log_ui(&mut log_node);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_exists() {
        let mut app = App::new();
        app.init_resource::<GlobalResources>();
        app.add_message::<crate::events::GameLogMessage>();
        app.add_plugins(UiPlugin);

        app.finish();
        app.cleanup();
        app.update();

        // Проверяем, что узлы интерфейса созданы (ищем по фоновому цвету или нодам)
        let mut query = app.world_mut().query::<&Node>();
        assert!(
            query.iter(app.world()).count() > 0,
            "UI Nodes should be spawned"
        );
    }
}
