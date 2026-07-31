use crate::economy::GlobalResources;
use crate::game_state::GameState;
use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;

pub mod details;
pub mod editor_ui;
pub mod logs;
pub mod panels;
pub mod resources;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            editor_ui::EditorUiPlugin,
            resources::ResourceUiPlugin,
            details::DetailUiPlugin,
            logs::GameLogPlugin,
            panels::tools_sub::ToolsSubPlugin,
        ));

        app.add_systems(Startup, setup_ui.in_set(StartupSet::SpawnEntities))
            .add_systems(
                Update,
                (
                    resources::update_resource_ui
                        .run_if(resource_changed::<GlobalResources>)
                        .in_set(GameSet::Visuals),
                    details::update_settler_detail_ui.in_set(GameSet::Visuals),
                    toggle_gameplay_hud
                        .run_if(state_changed::<GameState>)
                        .in_set(GameSet::Visuals),
                ),
            );
    }
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
        Visibility::Hidden,
        GameplayHud,
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
        Visibility::Hidden,
        GameplayHud,
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

#[derive(Component)]
pub struct GameplayHud;

fn toggle_gameplay_hud(
    state: Res<State<GameState>>,
    mut q_hud: Query<&mut Visibility, With<GameplayHud>>,
) {
    let vis = if *state.get() == GameState::Playing {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut q_hud {
        *v = vis;
    }
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

        let mut query = app.world_mut().query::<&Node>();
        assert!(
            query.iter(app.world()).count() > 0,
            "UI Nodes should be spawned"
        );
    }
}
