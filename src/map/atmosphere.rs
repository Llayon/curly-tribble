use bevy::prelude::*;
use crate::pawn::{Settler, Morale}; 

#[derive(Component)]
pub struct LightSource {
    pub radius: f32,
}

#[derive(Component, Default)]
pub struct Campfire;

#[derive(Bundle, Default)]
pub struct CampfireBundle {
    pub campfire: Campfire,
    pub transform: Transform,
}

/// Внутренний бандл для компонентов, добавляемых хуком Campfire.
#[derive(Bundle)]
struct CampfireLightBundle {
    pub light: PointLight,
    pub source: LightSource,
}

#[derive(Component)]
pub struct InDarkness; // Метка: находится вне света (беспокойство)

pub struct AtmospherePlugin;

impl Plugin for AtmospherePlugin {
    fn build(&self, app: &mut App) {
        // Регистрация хуков через World (Bevy 0.18.1)
        app.world_mut().register_component_hooks::<Campfire>()
            .on_add(|mut world, context| {
                let entity = context.entity;
                // Используем именованный Bundle вместо кортежа
                world.commands().entity(entity).insert(CampfireLightBundle {
                    light: PointLight {
                        shadows_enabled: true,
                        intensity: 1_000_000.0,
                        range: 20.0,
                        color: Color::srgb(1.0, 0.6, 0.3),
                        ..default()
                    },
                    source: LightSource { radius: 8.0 },
                });
            });

        app.add_systems(FixedUpdate, (
            detect_darkness,
            apply_darkness_effects,
        )
            .in_set(crate::sets::GameSet::Logic)
        );
    }
}

fn detect_darkness(
    mut commands: Commands,
    // Оптимизация: проверяем только если поселенец сдвинулся
    settlers: Query<(Entity, &Transform), (With<Settler>, Changed<Transform>)>,
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
    time: Res<Time<Fixed>>,
    mut query: Query<&mut Morale, (With<Settler>, With<InDarkness>)>,
    mut light_query: Query<&mut Morale, (With<Settler>, Without<InDarkness>)>,
) {
    // Тьма вызывает беспокойство и снижает мораль
    for mut morale in &mut query {
        morale.0 = (morale.0 - 1.0 * time.delta_secs()).max(0.0);
    }

    // Близость к костру возвращает уверенность
    for mut morale in &mut light_query {
        morale.0 = (morale.0 + 0.5 * time.delta_secs()).min(100.0);
    }
}
