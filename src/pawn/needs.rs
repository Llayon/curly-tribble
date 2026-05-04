use bevy::prelude::*;
use crate::game_state::GameState;
use super::{Hunger, Settler, Hungry};

pub struct NeedsPlugin;

impl Plugin for NeedsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            update_hunger,
            manage_hungry_marker,
        )
            .run_if(in_state(GameState::Playing))
            .in_set(crate::sets::GameSet::Logic)
        );
    }
}

fn update_hunger(time: Res<Time>, mut query: Query<&mut Hunger, With<Settler>>) {
    for mut hunger in &mut query {
        hunger.0 += 1.0 * time.delta_secs();
    }
}

fn manage_hungry_marker(
    mut commands: Commands,
    query: Query<(Entity, &Hunger), Without<Hungry>>,
    hungry_query: Query<(Entity, &Hunger), With<Hungry>>,
) {
    for (entity, hunger) in &query {
        if hunger.0 > 50.0 {
            commands.entity(entity).insert(Hungry);
        }
    }

    for (entity, hunger) in &hungry_query {
        if hunger.0 < 10.0 {
            commands.entity(entity).remove::<Hungry>();
        }
    }
}
