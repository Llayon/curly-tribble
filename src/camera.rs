use crate::game_state::GameState;
use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CameraFocus>()
            .register_type::<CameraConfig>()
            .add_systems(Startup, setup_camera.in_set(StartupSet::SpawnEntities))
            .add_systems(
                Update,
                move_camera
                    .run_if(in_state(GameState::Playing))
                    .in_set(GameSet::Input),
            );
    }
}

#[derive(Component, Reflect, Default)]
#[reflect(Component)]
pub struct CameraFocus(pub Vec3);

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct CameraConfig {
    pub distance: f32,
    pub azimuth: f32, // Rotation around Y
    pub pitch: f32,   // Tilt angle
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            distance: 15.0,
            azimuth: 0.0,
            pitch: 0.8, // Radian (~45 deg)
        }
    }
}

#[derive(Bundle)]
pub struct OrbitCameraBundle {
    pub camera: Camera3d,
    pub transform: Transform,
    pub focus: CameraFocus,
    pub config: CameraConfig,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(OrbitCameraBundle {
        camera: Camera3d::default(),
        transform: Transform::from_xyz(0.0, 15.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        focus: CameraFocus(Vec3::ZERO),
        config: CameraConfig::default(),
    });
}

fn move_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera3d>>,
    time: Res<Time>,
) {
    let Some(mut transform) = query.iter_mut().next() else {
        return;
    };
    let speed = 10.0;
    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction.z -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.z += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    transform.translation += direction.normalize_or_zero() * speed * time.delta_secs();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_panning_w() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.insert_resource(Time::<Real>::default());

        let entity = app
            .world_mut()
            .spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 0.0)))
            .id();

        // Mock pressing W
        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.press(KeyCode::KeyW);

        // Mock time delta
        let mut time = app.world_mut().resource_mut::<Time<Real>>();
        time.advance_by(std::time::Duration::from_secs_f32(0.1));

        app.add_systems(Update, move_camera);
        app.update();

        let transform = app.world().get::<Transform>(entity).unwrap();
        // Speed is 10.0, time is 0.1, direction is -Z
        assert_eq!(transform.translation.z, -1.0);
    }
}
