use bevy::prelude::*;
use crate::events::{GameLogMessage, LogSeverity};
use crate::economy::GameAssets;
use crate::sets::StartupSet;

pub mod needs;
pub mod brain;

#[derive(Component)]
pub struct Settler; // Метка человека-поселенца

#[derive(Component)]
pub struct Pioneer; // Роль: Пионер (первый из прибывших)

#[derive(Component)]
pub struct Lantern; // Метка фонаря

#[derive(Bundle)]
pub struct LanternBundle {
    pub lantern: Lantern,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub light: PointLight,
}

#[derive(Component, Default)]
pub struct Selected;

pub struct PawnPlugin;

impl Plugin for PawnPlugin {
    fn build(&self, app: &mut App) {
        // Регистрация хуков для Первопроходца
        app.world_mut().register_component_hooks::<Pioneer>()
            .on_add(|mut world, context| {
                let entity = context.entity;
                let assets = world.resource::<GameAssets>();
                let lantern_mesh = assets.lantern_mesh.clone();
                let lantern_material = assets.lantern_material.clone();

                world.commands().entity(entity).with_children(|parent| {
                    parent.spawn(LanternBundle {
                        lantern: Lantern,
                        mesh: Mesh3d(lantern_mesh),
                        material: MeshMaterial3d(lantern_material),
                        transform: Transform::from_xyz(0.4, 0.5, 0.3), // Позиция в "руке"
                        light: PointLight {
                            intensity: 100_000.0,
                            range: 10.0,
                            color: Color::srgb(1.0, 0.8, 0.4),
                            shadows_enabled: true,
                            ..default()
                        },
                    });
                });
            });

        app.add_plugins((needs::NeedsPlugin, brain::BrainPlugin))
           .add_systems(Startup, spawn_starting_settler.in_set(StartupSet::SpawnEntities));
    }
}

#[derive(Component)]
pub struct Hungry; // Состояние: нуждается в пище

#[derive(Component)]
pub struct Hunger(pub f32); // Уровень голода

#[derive(Component)]
pub struct Morale(pub f32); // Боевой дух: 100.0 (решимость) -> 0.0 (уныние)

#[derive(Bundle)]
pub struct SettlerBundle {
    pub settler: Settler,
    pub pioneer: Pioneer,
    pub hunger: Hunger,
    pub morale: Morale,
    pub name: Name,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

fn spawn_starting_settler(
    mut commands: Commands,
    assets: Res<GameAssets>,
) {
    commands.spawn(SettlerBundle {
        settler: Settler,
        pioneer: Pioneer,
        hunger: Hunger(0.0),
        morale: Morale(100.0),
        name: Name::new("Erik the Red"),
        mesh: Mesh3d(assets.settler_mesh.clone()),
        material: MeshMaterial3d(assets.settler_material.clone()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
    })
    .observe(|event: On<Pointer<Click>>, mut commands: Commands, selected: Query<Entity, With<Selected>>, mut messages: MessageWriter<GameLogMessage>| {
        for entity in &selected {
            commands.entity(entity).remove::<Selected>();
        }
        commands.entity(event.entity).insert(Selected);
        
        messages.write(GameLogMessage {
            message: "Survivor selected. They look tired but determined.".to_string(),
            severity: LogSeverity::Info,
        });
    })
    .observe(|trigger: On<Add, Selected>, mut query: Query<&mut MeshMaterial3d<StandardMaterial>, With<Settler>>, assets: Res<GameAssets>| {
        if let Ok(mut mat_handle) = query.get_mut(trigger.entity) {
             *mat_handle = MeshMaterial3d(assets.settler_selected_material.clone());
        }
    })
    .observe(|trigger: On<Remove, Selected>, mut query: Query<&mut MeshMaterial3d<StandardMaterial>, With<Settler>>, assets: Res<GameAssets>| {
        if let Ok(mut mat_handle) = query.get_mut(trigger.entity) {
             *mat_handle = MeshMaterial3d(assets.settler_material.clone());
        }
    });
}
