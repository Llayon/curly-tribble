use crate::economy::GlobalResources;
use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;

pub mod details;
pub mod logs;
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
    }
}

fn setup_ui(mut commands: Commands) {
    // 0. Explicit 2D Camera for UI.
    // Camera2d automatically adds the core Camera component, but we add it manually to set the order.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1, // UI рисуется поверх 3D мира (order 0)
            ..default()
        },
    ));

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
    fn test_ui_spawns_camera2d() {
        let mut app = App::new();
        app.init_resource::<GlobalResources>();
        app.add_message::<crate::events::GameLogMessage>();
        app.add_plugins(UiPlugin);

        // Run the app's startup systems
        app.finish();
        app.cleanup();
        app.update();

        let mut query = app.world_mut().query::<&Camera2d>();
        assert_eq!(
            query.iter(app.world()).count(),
            1,
            "UI should spawn exactly one Camera2d for rendering"
        );
    }
}
