use bevy::prelude::*;
use bevy_ai_remote::BevyAiRemotePlugin;

mod camera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Axiom integration plugin
        .add_plugins(BevyAiRemotePlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Light (shadows disabled for debugging)
    commands.spawn((
        PointLight {
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ground plane (so we have something to see)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(10.0, 10.0))),
        MeshMaterial3d(materials.add(Color::from(LinearRgba::from_f32_array([0.3, 0.5, 0.3, 1.0])))),
    ));
}
