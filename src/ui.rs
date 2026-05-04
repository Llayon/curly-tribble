use bevy::prelude::*;
use crate::economy::GlobalResources;

use crate::events::{GameLogMessage, LogSeverity};

use crate::sets::GameSet;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
           .add_systems(Update, (
               update_resource_ui, 
               handle_game_logs
           ).in_set(GameSet::Visuals));
    }
}

fn handle_game_logs(mut messages: MessageReader<GameLogMessage>) {
    // В Bevy 0.18.1 используем MessageReader
    for message in messages.read() {
        match message.severity {
            LogSeverity::Info => info!("[LOG] {}", message.message),
            LogSeverity::Warning => warn!("[WARN] {}", message.message),
            LogSeverity::DarkEvent => error!("[DARK] {}", message.message),
        }
    }
}

#[derive(Component)]
struct ResourceText;

fn setup_ui(mut commands: Commands) {
    // Root node for the top-left UI
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Food: 0"),
            TextFont {
                font_size: 24.0,
                ..default()
            },
            TextColor(Color::WHITE),
            ResourceText,
        ));
    });
}

fn update_resource_ui(
    resources: Res<GlobalResources>,
    mut query: Query<&mut Text, With<ResourceText>>,
) {
    for mut text in &mut query {
        text.0 = format!("Food: {:.0}", resources.food);
    }
}
