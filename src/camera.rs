use crate::game_state::GameState;
use crate::sets::{GameSet, StartupSet};
use bevy::anti_alias::taa::TemporalAntiAliasPlugin;
use bevy::anti_alias::taa::TemporalAntiAliasing;
use bevy::core_pipeline::core_3d::graph::Core3d;
use bevy::core_pipeline::prepass::{DepthPrepass, MotionVectorPrepass, NormalPrepass};
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::pbr::ScreenSpaceAmbientOcclusion;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::camera::CameraRenderGraph;
use bevy::render::view::Hdr;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TemporalAntiAliasPlugin>() {
            app.add_plugins(TemporalAntiAliasPlugin);
        }

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

fn setup_camera(mut commands: Commands) {
    // В Bevy 0.18.1 мы используем максимально явную инициализацию,
    // чтобы гарантировать привязку RenderGraph.
    commands.spawn((
        Camera3d::default(),
        CameraRenderGraph::new(Core3d),
        Transform::from_xyz(0.0, 15.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraFocus(Vec3::ZERO),
        CameraConfig::default(),
        Tonemapping::TonyMcMapface,
        Bloom::NATURAL,
        Hdr,
        Msaa::Off,
        DepthPrepass,
        NormalPrepass,
        MotionVectorPrepass,
        TemporalAntiAliasing::default(),
        ScreenSpaceAmbientOcclusion::default(),
        Name::new("Main Camera"),
    ));
}

fn move_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    mut query: Query<(&mut Transform, &mut CameraFocus, &mut CameraConfig), With<Camera3d>>,
    time: Res<Time>,
) {
    let Some((mut transform, mut focus, mut config)) = query.iter_mut().next() else {
        return;
    };

    // 1. Rotation (Q/E)
    let rot_speed = 2.0;
    if keyboard.pressed(KeyCode::KeyQ) {
        config.azimuth += rot_speed * time.delta_secs();
    }
    if keyboard.pressed(KeyCode::KeyE) {
        config.azimuth -= rot_speed * time.delta_secs();
    }

    // 2. Zoom (Mouse Wheel)
    for event in mouse_wheel.read() {
        config.distance -= event.y * 2.0;
        config.distance = config.distance.clamp(5.0, 40.0);
        config.pitch = (config.distance / 40.0 * 0.7 + 0.5).clamp(0.5, 1.2);
    }

    // 3. Movement (WASD) - Relative to camera rotation
    let move_speed = 15.0;
    let mut move_dir = Vec3::ZERO;

    let forward = Vec3::new(config.azimuth.sin(), 0.0, config.azimuth.cos()).normalize_or_zero();
    let right = Vec3::new(forward.z, 0.0, -forward.x);

    if keyboard.pressed(KeyCode::KeyW) {
        move_dir -= forward;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        move_dir += forward;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        move_dir -= right;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        move_dir += right;
    }

    focus.0 += move_dir.normalize_or_zero() * move_speed * time.delta_secs();

    // 4. Update Transform (Orbit math)
    let x = config.distance * config.azimuth.sin() * config.pitch.cos();
    let y = config.distance * config.pitch.sin();
    let z = config.distance * config.azimuth.cos() * config.pitch.cos();

    transform.translation = focus.0 + Vec3::new(x, y, z);
    transform.look_at(focus.0, Vec3::Y);
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
        app.add_message::<bevy::input::mouse::MouseWheel>();

        let entity = app
            .world_mut()
            .spawn((
                Camera3d::default(),
                CameraRenderGraph::new(Core3d),
                Transform::from_xyz(0.0, 0.0, 0.0),
                CameraFocus(Vec3::ZERO),
                CameraConfig::default(),
                Tonemapping::None,
                Bloom::default(),
                Hdr,
                Msaa::Off,
                DepthPrepass,
                NormalPrepass,
                MotionVectorPrepass,
                TemporalAntiAliasing::default(),
                ScreenSpaceAmbientOcclusion::default(),
            ))
            .id();

        let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        input.press(KeyCode::KeyW);

        let mut time = app.world_mut().resource_mut::<Time<Real>>();
        time.advance_by(std::time::Duration::from_secs_f32(0.1));

        app.add_systems(Update, move_camera);
        app.update();

        let focus = app.world().get::<CameraFocus>(entity).unwrap();
        assert!((focus.0.z - (-1.5)).abs() < 0.001);
    }
}
