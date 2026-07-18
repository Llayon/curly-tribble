use savage_fantasy::map::terrain_gen::{TerrainConfig, TerrainGenerator};
use savage_fantasy::map::MAX_HEIGHT;

#[test]
fn test_terrain_height_ranges() {
    let config = TerrainConfig::default();
    let generator = TerrainGenerator::new(config.seed);
    for (x, z) in [(0.0, 0.0), (12.5, -7.25), (-30.0, 18.0)] {
        let elevation = generator.get_elevation(&config, x, z);
        assert!(
            (0.0..=MAX_HEIGHT).contains(&elevation),
            "elevation at ({x}, {z}) must stay within the mesh height range"
        );
    }
}
