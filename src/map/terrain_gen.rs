// src/map/terrain_gen.rs
use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use noise::{Fbm, NoiseFn, Perlin};

#[derive(Resource, Reflect, InspectorOptions)]
#[reflect(Resource, InspectorOptions)]
pub struct TerrainConfig {
    pub map_width: u32,
    pub map_height: u32,
    pub seed: u32,
    #[inspector(min = 0.001, max = 0.1)]
    pub macro_freq: f64,
    #[inspector(min = 1.0, max = 25.0)]
    pub macro_height: f32,
    #[inspector(min = 1.0, max = 10.0)]
    pub macro_sharpness: f32,
    #[inspector(min = 0.001, max = 0.1)]
    pub plateau_freq: f64,
    #[inspector(min = 1.0, max = 10.0)]
    pub plateau_height: f32,
    #[inspector(min = 1.0, max = 10.0)]
    pub plateau_steps: f32,
    #[inspector(min = 0.001, max = 0.1)]
    pub warp_freq: f64,
    #[inspector(min = 0.0, max = 20.0)]
    pub warp_strength: f32,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            map_width: 120,
            map_height: 120,
            seed: 42,
            macro_freq: 0.03,
            macro_height: 12.0,
            macro_sharpness: 4.0,
            plateau_freq: 0.04,
            plateau_height: 4.0,
            plateau_steps: 4.0,
            warp_freq: 0.02,
            warp_strength: 5.0,
        }
    }
}

// Internal constant not exposed in config for now
const PASS_FREQ: f64 = 0.01;

#[derive(Resource)]
pub struct TerrainGenerator {
    macro_noise: Fbm<Perlin>,
    pass_noise: Perlin,
    plateau_noise: Fbm<Perlin>,
    warp_noise_x: Perlin,
    warp_noise_z: Perlin,
}

impl TerrainGenerator {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            macro_noise: Fbm::<Perlin>::new(seed),
            pass_noise: Perlin::new(seed + 1),
            plateau_noise: Fbm::<Perlin>::new(seed + 2),
            warp_noise_x: Perlin::new(seed + 3),
            warp_noise_z: Perlin::new(seed + 4),
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Noise output f64 to f32 is intentional for mesh attributes
    pub fn get_elevation(&self, config: &TerrainConfig, x: f32, z: f32) -> f32 {
        let x64 = f64::from(x);
        let z64 = f64::from(z);

        // 1. MACRO: Ridged Mountains with Pass Mask
        let ridge_val = self.macro_noise.get([x64 * config.macro_freq, z64 * config.macro_freq]);
        let ridge = (1.0 - ridge_val.abs()).powf(f64::from(config.macro_sharpness)) as f32;

        let pass_val = self.pass_noise.get([x64 * PASS_FREQ, z64 * PASS_FREQ]);
        let pass_mask = ((pass_val + 1.0) * 0.5) as f32;

        let mountains = ridge * pass_mask * config.macro_height;

        // 2. MICRO: Domain Warped Terraced Plateaus
        let wx = self.warp_noise_x.get([x64 * config.warp_freq, z64 * config.warp_freq]) as f32 * config.warp_strength;
        let wz = self
            .warp_noise_z
            .get([x64 * config.warp_freq + 100.0, z64 * config.warp_freq + 100.0]) as f32
            * config.warp_strength;

        let plateau_val = self.plateau_noise.get([
            (x64 + f64::from(wx)) * config.plateau_freq,
            (z64 + f64::from(wz)) * config.plateau_freq,
        ]);
        let plateau_base = ((plateau_val + 1.0) * 0.5) as f32;

        let plateaus = Self::smoothstep_terracing(plateau_base, config.plateau_steps) * config.plateau_height;

        // 3. BLENDING: Max() for predictable range
        mountains.max(plateaus)
    }

    fn smoothstep_terracing(val: f32, steps: f32) -> f32 {
        let scaled = val * steps;
        let floor_val = scaled.floor();
        let fract_val = scaled - floor_val;

        // Cubic Hermite Interpolation (Smoothstep) for passable slopes
        let smoothed_fract = fract_val * fract_val * (3.0 - 2.0 * fract_val);
        (floor_val + smoothed_fract) / steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_generator_elevation() {
        let config = TerrainConfig::default();
        let gen = TerrainGenerator::new(config.seed);
        let e1 = gen.get_elevation(&config, 0.0, 0.0);
        let e2 = gen.get_elevation(&config, 10.0, 10.0);

        // Check that it produces some value and is deterministic
        assert!((e1 - gen.get_elevation(&config, 0.0, 0.0)).abs() < f32::EPSILON);
        assert!(e1 >= 0.0);
        assert!(e2 >= 0.0);
    }
}
