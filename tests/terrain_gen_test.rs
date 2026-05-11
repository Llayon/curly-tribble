use savage_fantasy::map::terrain_gen::TerrainGenerator;

#[test]
fn test_terrain_height_ranges() {
    let gen = TerrainGenerator::new(42);
    for x in -100..100 {
        for z in -100..100 {
            let h = gen.get_elevation(x as f32, z as f32);
            assert!(h >= 0.0 && h <= 12.0, "Height {} out of range at ({}, {})", h, x, z);
        }
    }
}
