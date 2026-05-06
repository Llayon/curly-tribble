use bevy::prelude::*;
use crate::sets::StartupSet;

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

fn setup_economy(
    mut commands: Commands,
    mut resources: ResMut<GlobalResources>,
) {
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

pub trait EconomyCommandsExt {
    fn consume_food(&mut self, amount: f32) -> &mut Self;
    fn add_food(&mut self, amount: f32) -> &mut Self;
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
}
