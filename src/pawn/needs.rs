use bevy::prelude::*;
use super::{Hunger, Settler, Hungry};

pub struct NeedsPlugin;

impl Plugin for NeedsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(FixedUpdate, (
            update_hunger,
            manage_hungry_marker,
        )
            .in_set(crate::sets::GameSet::Logic)
        );
    }
}

fn update_hunger(time: Res<Time<Fixed>>, mut query: Query<&mut Hunger, With<Settler>>) {
    // В FixedUpdate используем Time<Fixed> для абсолютной точности
    for mut hunger in &mut query {
        hunger.increase(1.0 * time.delta_secs());
    }
}

fn manage_hungry_marker(
    mut commands: Commands,
    // Проверяем только тех, чей уровень голода изменился
    query: Query<(Entity, &Hunger), (Without<Hungry>, Changed<Hunger>)>,
    hungry_query: Query<(Entity, &Hunger), (With<Hungry>, Changed<Hunger>)>,
) {
    for (entity, hunger) in &query {
        if hunger.value() > 50.0 {
            commands.entity(entity).insert(Hungry);
        }
    }

    for (entity, hunger) in &hungry_query {
        if hunger.value() < 10.0 {
            commands.entity(entity).remove::<Hungry>();
        }
    }
}
