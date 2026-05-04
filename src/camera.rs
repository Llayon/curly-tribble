use bevy::prelude::*;

use crate::sets::GameSet;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_camera)
           .add_systems(Update, move_camera.in_set(GameSet::Input));
    }
}

#[derive(Bundle)]
pub struct GameCameraBundle {
    pub camera: Camera3d,
    pub transform: Transform,
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(GameCameraBundle {
        camera: Camera3d::default(),
        transform: Transform::from_xyz(0.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    });
}

fn move_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Camera>>,
    time: Res<Time>,
) {
    let mut transform = if let Some(t) = query.iter_mut().next() { t } else { return; };
    let speed = 10.0;
    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) { direction.z -= 1.0; }
    if keyboard.pressed(KeyCode::KeyS) { direction.z += 1.0; }
    if keyboard.pressed(KeyCode::KeyA) { direction.x -= 1.0; }
    if keyboard.pressed(KeyCode::KeyD) { direction.x += 1.0; }
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
        
        let entity = app.world_mut().spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 0.0),
        )).id();
        
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
