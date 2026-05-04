use bevy::prelude::*;
use crate::events::{GameLogMessage, LogSeverity};
use crate::economy::GameAssets;

pub mod needs;
pub mod brain;

#[derive(Component)]
pub struct Settler; // Метка человека-поселенца

#[derive(Component)]
pub struct Pioneer; // Роль: Пионер (первый из прибывших)

#[derive(Component)]
pub struct Hungry; // Состояние: нуждается в пище

#[derive(Component)]
pub struct Hunger(pub f32); // Уровень голода

#[derive(Component)]
pub struct Sanity(pub f32); // Уровень рассудка: 100.0 (стабилен) -> 0.0 (паника)

#[derive(Bundle)]
pub struct SettlerBundle {
    pub settler: Settler,
    pub pioneer: Pioneer,
    pub hunger: Hunger,
    pub sanity: Sanity,
    pub name: Name,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
}

#[derive(Component, Default)]
pub struct Selected;

pub struct PawnPlugin;

impl Plugin for PawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((needs::NeedsPlugin, brain::BrainPlugin))
           .add_systems(Startup, spawn_starting_settler);
    }
}

fn spawn_starting_settler(
    mut commands: Commands,
    assets: Res<GameAssets>, // Используем кэш ассетов
) {
    // Спавним Эрика Рыжего - первого выжившего
    commands.spawn(SettlerBundle {
        settler: Settler,
        pioneer: Pioneer,
        hunger: Hunger(0.0),
        sanity: Sanity(100.0),
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
             *mat_handle = MeshMaterial3d(assets.settler_selected_material.clone()); // Используем кэшированный материал
        }
    })
    .observe(|trigger: On<Remove, Selected>, mut query: Query<&mut MeshMaterial3d<StandardMaterial>, With<Settler>>, assets: Res<GameAssets>| {
        if let Ok(mut mat_handle) = query.get_mut(trigger.entity) {
             *mat_handle = MeshMaterial3d(assets.settler_material.clone()); // Возвращаем исходный из кэша
        }
    });
}
