use bevy::prelude::*;

/// Сообщение (буферизованное событие) для записи в историю игры.
/// В Bevy 0.18.1 традиционные события теперь называются Messages.
#[derive(Message)]
pub struct GameLogMessage {
    pub message: String,
    pub severity: LogSeverity,
}

pub enum LogSeverity {
    Info,
    Warning,
    DarkEvent,
}

pub struct EventsPlugin;

impl Plugin for EventsPlugin {
    fn build(&self, app: &mut App) {
        // В Bevy 0.18.1 регистрация буферизованных событий
        app.add_message::<GameLogMessage>();
    }
}
