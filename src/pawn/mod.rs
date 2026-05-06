use crate::economy::GameAssets;
use crate::events::{GameLogMessage, LogSeverity};
use crate::sets::StartupSet;
use bevy::prelude::*;

pub mod behaviors;
pub mod brain;
pub mod needs;
pub mod relations;

use behaviors::{BehaviorExt, Idle};
use relations::RelationsPlugin;

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
        app.world_mut()
            .register_component_hooks::<Pioneer>()
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

        app.add_plugins((
            needs::NeedsPlugin,
            brain::BrainPlugin,
            behaviors::BehaviorsPlugin,
            RelationsPlugin,
        ))
        .add_systems(
            Startup,
            spawn_starting_settler.in_set(StartupSet::SpawnEntities),
        );
    }
}

#[derive(Component)]
pub struct Hungry; // Состояние: нуждается в пище

#[derive(Component, Debug, Clone, Copy)]
pub struct Hunger(f32); // Уровень голода (0 - сыт, 100 - истощен)

impl Hunger {
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 100.0))
    }
    pub fn value(self) -> f32 {
        self.0
    }
    #[allow(dead_code)]
    pub fn is_starving(self) -> bool {
        self.0 >= 90.0
    }
    pub fn increase(&mut self, amount: f32) {
        self.0 = (self.0 + amount).min(100.0);
    }
    #[allow(dead_code)]
    pub fn satisfy(&mut self, amount: f32) {
        self.0 = (self.0 - amount).max(0.0);
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct Morale(f32); // Боевой дух (100 - решимость, 0 - уныние)

impl Morale {
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 100.0))
    }
    pub fn value(self) -> f32 {
        self.0
    }
    pub fn add(&mut self, amount: f32) {
        self.0 = (self.0 + amount).min(100.0);
    }
    pub fn reduce(&mut self, amount: f32) {
        self.0 = (self.0 - amount).max(0.0);
    }
}

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

fn spawn_starting_settler(mut commands: Commands, assets: Res<GameAssets>) {
    let mut settler = commands.spawn(SettlerBundle {
        settler: Settler,
        pioneer: Pioneer,
        hunger: Hunger::new(0.0),
        morale: Morale::new(100.0),
        name: Name::new("Erik the Red"),
        mesh: Mesh3d(assets.settler_mesh.clone()),
        material: MeshMaterial3d(assets.settler_material.clone()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
    });

    // Инициализируем стартовое поведение через безопасный переключатель
    settler.switch_behavior::<Idle>();

    settler
        .observe(
            |event: On<Pointer<Click>>,
             mut commands: Commands,
             selected: Query<Entity, With<Selected>>,
             mut messages: MessageWriter<GameLogMessage>| {
                for entity in &selected {
                    commands.entity(entity).remove::<Selected>();
                }
                commands.entity(event.entity).insert(Selected);

                messages.write(GameLogMessage {
                    message: "Survivor selected. They look tired but determined.".to_string(),
                    severity: LogSeverity::Info,
                });
            },
        )
        .observe(
            |trigger: On<Add, Selected>,
             mut query: Query<&mut MeshMaterial3d<StandardMaterial>, With<Settler>>,
             assets: Res<GameAssets>| {
                if let Ok(mut mat_handle) = query.get_mut(trigger.entity) {
                    *mat_handle = MeshMaterial3d(assets.settler_selected_material.clone());
                }
            },
        )
        .observe(
            |trigger: On<Remove, Selected>,
             mut query: Query<&mut MeshMaterial3d<StandardMaterial>, With<Settler>>,
             assets: Res<GameAssets>| {
                if let Ok(mut mat_handle) = query.get_mut(trigger.entity) {
                    *mat_handle = MeshMaterial3d(assets.settler_material.clone());
                }
            },
        );
}
