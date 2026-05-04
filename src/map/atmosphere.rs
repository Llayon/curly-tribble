use bevy::prelude::*;
use crate::pawn::{Settler, Sanity}; // Убрали Hungry

#[derive(Component)]
pub struct LightSource {
    pub radius: f32,
}

#[derive(Component)]
pub struct InDarkness; // Метка: находится во тьме

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            detect_darkness,
            apply_darkness_effects,
        )
            .run_if(in_state(crate::game_state::GameState::Playing))
            .in_set(crate::sets::GameSet::Logic)
        );
    }
}

fn detect_darkness(
    mut commands: Commands,
    settlers: Query<(Entity, &Transform), With<Settler>>,
    lights: Query<(&Transform, &LightSource)>,
) {
    for (settler_entity, settler_transform) in &settlers {
        let mut illuminated = false;
        
        for (light_transform, light_source) in &lights {
            let distance = settler_transform.translation.distance(light_transform.translation);
            if distance < light_source.radius {
                illuminated = true;
                break;
            }
        }

        if illuminated {
            commands.entity(settler_entity).remove::<InDarkness>();
        } else {
            commands.entity(settler_entity).insert(InDarkness);
        }
    }
}

fn apply_darkness_effects(
    time: Res<Time>,
    mut query: Query<&mut Sanity, (With<Settler>, With<InDarkness>)>,
    mut light_query: Query<&mut Sanity, (With<Settler>, Without<InDarkness>)>,
) {
    // Тьма пожирает рассудок
    for mut sanity in &mut query {
        sanity.0 = (sanity.0 - 2.0 * time.delta_secs()).max(0.0);
    }

    // Свет возвращает надежду
    for mut sanity in &mut light_query {
        sanity.0 = (sanity.0 + 1.0 * time.delta_secs()).min(100.0);
    }
}
