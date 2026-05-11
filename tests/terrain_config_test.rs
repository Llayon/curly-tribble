// tests/terrain_config_test.rs
use bevy::prelude::*;
use savage_fantasy::map::terrain_gen::{TerrainConfig, TerrainGenerator};

#[test]
fn test_terrain_config_integration() {
    let mut app = App::new();
    let config = TerrainConfig::default();
    
    // Test default values
    assert_eq!(config.seed, 42);
    assert_eq!(config.map_width, 120);
    
    app.insert_resource(config);
    app.insert_resource(TerrainGenerator::new(42));
    
    let world_config = app.world().get_resource::<TerrainConfig>().expect("TerrainConfig resource missing");
    let gen = app.world().get_resource::<TerrainGenerator>().expect("TerrainGenerator resource missing");
    
    let h = gen.get_elevation(world_config, 0.0, 0.0);
    assert!(h >= 0.0);
}
