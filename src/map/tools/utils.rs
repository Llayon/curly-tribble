use bevy::prelude::*;

pub struct UtilsToolPlugin;

impl Plugin for UtilsToolPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn get_mouse_world_pos(
    q_camera: &Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: &Query<&Window, With<bevy::window::PrimaryWindow>>,
) -> Option<Vec3> {
    let Ok((camera, camera_transform)) = q_camera.single() else {
        return None;
    };
    let Ok(window) = q_window.single() else {
        return None;
    };

    let cursor_pos = window.cursor_position()?;
    let ray = camera
        .viewport_to_world(camera_transform, cursor_pos)
        .ok()?;

    let distance = ray.origin.y / -ray.direction.y;
    if distance <= 0.0 {
        return None;
    }

    Some(ray.origin + ray.direction * distance)
}
