use crate::economy::GameAssets;
use crate::sets::StartupSet;
use bevy::prelude::*;

pub struct ResourcesPlugin;

impl Plugin for ResourcesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_bushes.in_set(StartupSet::SpawnEntities));
    }
}

#[derive(Component)]
pub struct BerryBush {
    pub food_amount: f32,
}

#[derive(Bundle)]
pub struct BerryBushBundle {
    pub bush: BerryBush,
    pub mesh: Mesh3d,
    pub material: MeshMaterial3d<StandardMaterial>,
    pub transform: Transform,
    pub obstacle: crate::map::navigation::NavObstacle,
}

fn spawn_bushes(mut commands: Commands, assets: Res<GameAssets>) {
    // Рассадим несколько таинственных кустов вокруг берега
    let bush_positions = [
        Vec3::new(3.0, 0.4, 2.0),
        Vec3::new(-4.0, 0.4, -3.0),
        Vec3::new(2.0, 0.4, -4.0),
    ];

    for pos in bush_positions {
        commands.spawn(BerryBushBundle {
            bush: BerryBush { food_amount: 10.0 },
            mesh: Mesh3d(assets.bush_mesh.clone()),
            material: MeshMaterial3d(assets.bush_material.clone()),
            transform: Transform::from_translation(pos),
            obstacle: crate::map::navigation::NavObstacle {
                cost: crate::map::navigation::COST_BLOCKER,
            },
        });
    }
}
