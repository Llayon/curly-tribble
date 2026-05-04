use bevy::prelude::*;

/// Глобальное хранилище ассетов для предотвращения дублирования в памяти.
#[derive(Resource, Default)]
pub struct GameAssets {
    pub settler_mesh: Handle<Mesh>,
    pub settler_material: Handle<StandardMaterial>,
    pub settler_selected_material: Handle<StandardMaterial>,
    pub ground_mesh: Handle<Mesh>,
    pub ground_material: Handle<StandardMaterial>,
}

#[derive(Resource, Default)]
pub struct GlobalResources {
    pub food: f32,
}

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GlobalResources>()
           .init_resource::<GameAssets>()
           .add_systems(Startup, (setup_assets, setup_economy).chain());
    }
}

#[derive(Bundle)]
pub struct LightBundle {
    pub light: PointLight,
    pub transform: Transform,
}

fn setup_assets(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Предзагрузка всех мешей и материалов
    let assets = GameAssets {
        settler_mesh: meshes.add(Capsule3d::new(0.3, 1.0)),
        settler_material: materials.add(Color::srgb(0.9, 0.7, 0.6)),
        settler_selected_material: materials.add(Color::srgb(1.0, 1.0, 0.2)),
        ground_mesh: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
        ground_material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
    };
    
    commands.insert_resource(assets);
}

fn setup_economy(
    mut commands: Commands,
    mut resources: ResMut<GlobalResources>,
) {
    resources.food = 10.0;

    commands.spawn(LightBundle {
        light: PointLight {
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
    });
}
