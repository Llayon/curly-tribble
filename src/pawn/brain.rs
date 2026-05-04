use bevy::prelude::*;
use super::{Hunger, Settler, Hungry};
use crate::economy::GlobalResources;
use crate::events::{GameLogMessage, LogSeverity};
use crate::sets::GameSet;

pub struct BrainPlugin;

impl Plugin for BrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, think.in_set(GameSet::Logic));
    }
}

fn think(
    mut resources: ResMut<GlobalResources>, 
    mut query: Query<&mut Hunger, (With<Settler>, With<Hungry>)>,
    mut messages: MessageWriter<GameLogMessage>,
) {
    for mut hunger in &mut query {
        if resources.food > 0.0 {
            resources.food -= 1.0;
            hunger.0 = 0.0;
            
            messages.write(GameLogMessage {
                message: "A settler consumed their rations. Survival continues.".to_string(),
                severity: LogSeverity::Info,
            });
        }
    }
}
