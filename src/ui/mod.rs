use crate::economy::GlobalResources;
use crate::events::{GameLogMessage, LogSeverity};
use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;

pub mod details;
pub mod resources;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((resources::ResourceUiPlugin, details::DetailUiPlugin));

        app.add_systems(Startup, setup_ui.in_set(StartupSet::SpawnEntities))
            .add_systems(
                Update,
                (
                    resources::update_resource_ui
                        .run_if(resource_changed::<GlobalResources>)
                        .in_set(GameSet::Visuals),
                    handle_game_logs.in_set(GameSet::Visuals),
                    details::update_settler_detail_ui.in_set(GameSet::Visuals),
                ),
            );
    }
}

fn handle_game_logs(mut messages: MessageReader<GameLogMessage>) {
    for message in messages.read() {
        match message.severity {
            LogSeverity::Info => info!("[LOG] {}", message.message),
            LogSeverity::Warning => warn!("[WARN] {}", message.message),
            LogSeverity::DarkEvent => error!("[MYSTERY] {}", message.message),
        }
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
}
