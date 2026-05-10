use crate::pawn::{Morale, Settler};
use crate::sets::StartupSet;
use bevy::light::{CascadeShadowConfig, CascadeShadowConfigBuilder};
use bevy::prelude::*;
use std::f32::consts::PI;

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

/// Внутренний бандл для автоматизации света Солнца (чтобы избежать анонимных кортежей)
#[derive(Bundle)]
pub struct SunBundle {
    pub light: DirectionalLight,
    pub transform: Transform,
    pub visibility: Visibility,
    pub shadow_config: CascadeShadowConfig,
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
        app.world_mut()
            .register_component_hooks::<Campfire>()
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

        app.add_systems(
            FixedUpdate,
            (detect_darkness, apply_darkness_effects).in_set(crate::sets::GameSet::Logic),
        );

        app.add_systems(Startup, setup_atmosphere.in_set(StartupSet::SpawnEntities));
    }
}

fn setup_atmosphere(mut commands: Commands) {
    // 2. Главный свет (Солнце) + CSM
    commands.spawn(SunBundle {
        light: DirectionalLight {
            shadows_enabled: true,
            illuminance: 3000.0, // Яркость солнца
            ..default()
        },
        transform: Transform {
            translation: Vec3::new(0.0, 50.0, 0.0),
            rotation: Quat::from_rotation_x(-PI / 4.0), // Под углом 45 градусов
            ..default()
        },
        visibility: Visibility::default(),
        // Настройка каскадных теней для больших пространств
        shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 4,
            minimum_distance: 0.1,
            maximum_distance: 150.0,
            first_cascade_far_bound: 10.0,
            overlap_proportion: 0.2,
            ..default()
        }
        .build(),
    });
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
            let distance = settler_transform
                .translation
                .distance(light_transform.translation);
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
        morale.reduce(1.0 * time.delta_secs());
    }

    // Близость к костру возвращает уверенность
    for mut morale in &mut light_query {
        morale.add(0.5 * time.delta_secs());
    }
}
