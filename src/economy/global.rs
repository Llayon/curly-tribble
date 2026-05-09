use crate::sets::StartupSet;
use bevy::prelude::*;

pub struct GlobalEconomyPlugin;

impl Plugin for GlobalEconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalResources>()
            .add_systems(Startup, setup_economy.in_set(StartupSet::LoadAssets));
    }
}

#[derive(Resource, Default)]
pub struct GlobalResources {
    pub food: f32,
}

fn setup_economy(mut commands: Commands, mut resources: ResMut<GlobalResources>) {
    resources.food = 10.0;

    commands.spawn(AmbientLight {
        color: Color::srgb(0.05, 0.05, 0.1),
        brightness: 100.0,
        ..default()
    });

    // "The Great Campfire"
    commands.spawn(crate::map::atmosphere::CampfireBundle {
        transform: Transform::from_xyz(0.0, 2.0, 0.0),
        ..default()
    });
}

// ============================================================================
// PLUGINS 2.0 (OPTIMIZED): NAMED COMMANDS & FLUENT API
// ============================================================================

/// Команда: Потребление ресурсов
#[allow(dead_code)]
pub struct ConsumeFood {
    pub amount: f32,
}

impl Command for ConsumeFood {
    fn apply(self, world: &mut World) {
        if let Some(mut resources) = world.get_resource_mut::<GlobalResources>() {
            resources.food = (resources.food - self.amount).max(0.0);
        }
    }
}

/// Команда: Пополнение ресурсов
pub struct AddFood {
    pub amount: f32,
}

impl Command for AddFood {
    fn apply(self, world: &mut World) {
        if let Some(mut resources) = world.get_resource_mut::<GlobalResources>() {
            resources.food += self.amount;
        }
    }
}

use crate::map::construction::{WardingStone, WardingStoneBundle};
use crate::map::navigation::NavObstacle;

/// Команда: Строительство Обережного Камня
pub struct BuildWardingStone {
    pub position: Vec3,
}

impl Command for BuildWardingStone {
    fn apply(self, world: &mut World) {
        let cost = 5.0;
        let mut enough_resources = false;
        if let Some(mut resources) = world.get_resource_mut::<GlobalResources>() {
            if resources.food >= cost {
                resources.food -= cost;
                enough_resources = true;
            }
        }

        if enough_resources {
            if let Some(assets) = world.get_resource::<crate::economy::assets::GameAssets>() {
                let scene = assets.house_scene.clone();

                world.commands().spawn(WardingStoneBundle {
                    stone: WardingStone,
                    scene: SceneRoot(scene),
                    transform: Transform::from_translation(self.position),
                    obstacle: NavObstacle::default(),
                });
            }
        }
    }
}

pub trait EconomyCommandsExt {
    #[allow(dead_code)]
    fn consume_food(&mut self, amount: f32) -> &mut Self;
    fn add_food(&mut self, amount: f32) -> &mut Self;
    fn build_warding_stone(&mut self, position: Vec3) -> &mut Self;
}

impl EconomyCommandsExt for Commands<'_, '_> {
    fn consume_food(&mut self, amount: f32) -> &mut Self {
        self.queue(ConsumeFood { amount });
        self
    }

    fn add_food(&mut self, amount: f32) -> &mut Self {
        self.queue(AddFood { amount });
        self
    }

    fn build_warding_stone(&mut self, position: Vec3) -> &mut Self {
        self.queue(BuildWardingStone { position });
        self
    }
}
