use bevy::prelude::*;
use savage_fantasy::economy::GameAssets;
use savage_fantasy::map::{MapData, MapPlugin};

#[test]
fn test_map_scaling_40x40() {
    let mut app = App::new();

    // Minimal Bevy setup
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_asset::<Mesh>();
    app.init_asset::<StandardMaterial>();
    app.init_asset::<Scene>();

    // Mock GameAssets
    let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
    let ground_mesh = meshes.add(Plane3d::default().mesh().size(1.0, 1.0));

    let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
    let ground_material = materials.add(Color::WHITE);

    app.insert_resource(GameAssets {
        ground_mesh,
        ground_material: ground_material.clone(),
        mud_material: ground_material.clone(),
        sand_material: ground_material.clone(),
        water_material: ground_material.clone(),
        stone_material: ground_material.clone(),
        ..Default::default()
    });

    app.add_plugins(MapPlugin);

    // Run startup systems
    app.update();

    let map_data = app.world().resource::<MapData>();
    assert_eq!(map_data.width, 40);
    assert_eq!(map_data.height, 40);
    assert_eq!(map_data.tiles.len(), 1600);
}
