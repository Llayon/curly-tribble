use bevy::prelude::*;
use super::{Hunger, Settler, Hungry};
use crate::economy::global::EconomyCommandsExt; // Импортируем наше расширение
use crate::events::{GameLogMessage, LogSeverity};
use crate::sets::GameSet;

pub struct BrainPlugin;

impl Plugin for BrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, 
            think.in_set(GameSet::Logic)
        );
    }
}

fn think(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Hunger), (With<Settler>, With<Hungry>)>,
    mut messages: MessageWriter<GameLogMessage>,
) {
    for (entity, mut hunger) in &mut query {
        // Мы НЕ используем ResMut<GlobalResources> здесь. 
        // Мы отдаем ПРИКАЗ системе экономики.
        if hunger.value() > 50.0 {
            commands.consume_food(1.0); // Просто и семантично
            
            hunger.satisfy(30.0);
            
            messages.write(GameLogMessage {
                message: "A settler consumed their rations. Survival continues.".to_string(),
                severity: LogSeverity::Info,
            });
        }
    }
}
