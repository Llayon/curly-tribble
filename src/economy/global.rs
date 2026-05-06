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
