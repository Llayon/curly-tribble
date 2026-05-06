use bevy::prelude::*;

// ============================================================================
// СИСТЕМА СТРОИТЕЛЬСТВА: Warding Stone
// ============================================================================

#[derive(Component, Default)]
pub struct WardingStone;

#[derive(Bundle, Default)]
pub struct WardingStoneBundle {
    pub stone: WardingStone,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

/// Внутренний бандл для автоматизации света Обережного Камня
#[derive(Bundle)]
struct WardingStoneLightBundle {
    pub light: PointLight,
    pub relation_target: crate::pawn::relations::Illuminating,
}

pub struct ConstructionPlugin;

impl Plugin for ConstructionPlugin {
    fn build(&self, app: &mut App) {
        // Регистрация хуков для Обережного Камня
        app.world_mut()
            .register_component_hooks::<WardingStone>()
            .on_add(|mut world, context| {
                let entity = context.entity;
                world
                    .commands()
                    .entity(entity)
                    .insert(WardingStoneLightBundle {
                        light: PointLight {
                            shadows_enabled: true,
                            intensity: 150_000.0,
                            range: 12.0,
                            color: Color::srgb(0.5, 0.8, 1.0), // Магический синий
                            ..default()
                        },
                        // Используем Default, так как Bevy сам заполнит коллекцию
                        relation_target: default(),
                    });
            });
    }
}
