use crate::sets::StartupSet;
use bevy::prelude::*;

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_assets.in_set(StartupSet::LoadAssets));
    }
}

/// Глобальное хранилище ассетов для предотвращения дублирования в памяти.
#[derive(Resource, Default)]
pub struct GameAssets {
    pub settler_mesh: Handle<Mesh>,
    pub settler_material: Handle<StandardMaterial>,
    pub settler_selected_material: Handle<StandardMaterial>,
    pub ground_mesh: Handle<Mesh>,
    pub ground_material: Handle<StandardMaterial>,
    pub mud_material: Handle<StandardMaterial>,
    pub sand_material: Handle<StandardMaterial>,
    pub water_material: Handle<StandardMaterial>,
    pub lantern_mesh: Handle<Mesh>,
    pub lantern_material: Handle<StandardMaterial>,
    pub bush_mesh: Handle<Mesh>,
    pub bush_material: Handle<StandardMaterial>,
    pub stone_mesh: Handle<Mesh>,
    pub stone_material: Handle<StandardMaterial>,
    // GLTF Scenes from Quaternius
    pub bush_scene: Handle<Scene>,
    pub tree_scene: Handle<Scene>,
    pub rock_scene: Handle<Scene>,
    pub house_scene: Handle<Scene>,
}

pub fn setup_assets(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    let assets = GameAssets {
        settler_mesh: meshes.add(Capsule3d::new(0.3, 1.0)),
        settler_material: materials.add(Color::srgb(0.9, 0.7, 0.6)),
        settler_selected_material: materials.add(Color::srgb(1.0, 1.0, 0.2)),
        ground_mesh: meshes.add(Plane3d::default().mesh().size(1.0, 1.0)),
        ground_material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        mud_material: materials.add(Color::srgb(0.4, 0.3, 0.2)),
        sand_material: materials.add(Color::srgb(0.8, 0.7, 0.4)),
        water_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.1, 0.3, 0.6),
            alpha_mode: AlphaMode::Blend,
            ..default()
        }),
        lantern_mesh: meshes.add(Cuboid::new(0.1, 0.2, 0.1)),
        lantern_material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 0.5),
            emissive: LinearRgba::from(Color::srgb(1.0, 0.8, 0.2)) * 5.0,
            ..default()
        }),
        // Таинственный ягодный куст: фиолетовый с золотым свечением
        bush_mesh: meshes.add(Sphere::new(0.4)),
        bush_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.2, 0.6),
            emissive: LinearRgba::from(Color::srgb(0.8, 0.6, 0.1)) * 2.0,
            ..default()
        }),
        // Обережный камень: высокий узкий кристалл (конус)
        stone_mesh: meshes.add(Cone::new(0.2, 0.8)),
        stone_material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.4, 0.8), // Синее свечение (защита)
            emissive: LinearRgba::from(Color::srgb(0.3, 0.6, 1.0)) * 4.0,
            ..default()
        }),
        // Загрузка сцен из GLTF
        bush_scene: asset_server.load("models/Resource_Tree2.gltf#Scene0"),
        tree_scene: asset_server.load("models/Resource_Tree1.gltf#Scene0"),
        rock_scene: asset_server.load("models/Resource_Rock_1.gltf#Scene0"),
        house_scene: asset_server.load("models/Houses_FirstAge_1_Level1.gltf#Scene0"),
    };

    commands.insert_resource(assets);
}
