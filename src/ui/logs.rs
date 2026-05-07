// src/ui/logs.rs
use crate::events::{GameLogMessage, LogSeverity};
use crate::sets::GameSet;
use bevy::prelude::*;

#[derive(Component)]
pub struct GameLogText;

pub struct GameLogPlugin;

impl Plugin for GameLogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_game_log.in_set(GameSet::Visuals));
    }
}

pub fn setup_log_ui(parent: &mut EntityCommands) {
    parent.with_children(|log_node| {
        log_node
            .spawn((
                Node {
                    padding: UiRect::all(Val::Px(10.0)),
                    max_width: Val::Px(400.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ))
            .with_children(|text_node| {
                text_node.spawn((
                    Text::new(""),
                    TextFont {
                        font_size: 16.0,
                        ..default()
                    },
                    GameLogText,
                ));
            });
    });
}

fn update_game_log(
    mut messages: MessageReader<GameLogMessage>,
    mut query: Query<&mut Text, With<GameLogText>>,
    mut buffer: Local<Vec<(String, LogSeverity)>>,
) {
    let mut changed = false;
    for msg in messages.read() {
        buffer.push((msg.message.clone(), msg.severity));
        if buffer.len() > 5 {
            buffer.remove(0);
        }
        changed = true;
    }

    if changed {
        if let Some(mut text) = query.iter_mut().next() {
            text.0 = String::new();
            for (msg, _sev) in buffer.iter() {
                text.0.push_str(&format!("{}\n", msg));
            }
        }
    }
}
