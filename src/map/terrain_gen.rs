// src/map/terrain_gen.rs
use bevy::prelude::*;
use noise::{Fbm, NoiseFn, Perlin};

pub const MACRO_FREQ: f64 = 0.03;
pub const PASS_FREQ: f64 = 0.01;
pub const MACRO_HEIGHT: f32 = 12.0;
pub const MACRO_SHARPNESS: f32 = 4.0;

pub const PLATEAU_FREQ: f64 = 0.04;
pub const PLATEAU_HEIGHT: f32 = 4.0;
pub const PLATEAU_STEPS: f32 = 4.0;
pub const WARP_FREQ: f64 = 0.02;
pub const WARP_STRENGTH: f32 = 5.0;

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
    #[allow(clippy::cast_possible_truncation)]
    pub fn get_elevation(&self, x: f32, z: f32) -> f32 {
        let x64 = f64::from(x);
        let z64 = f64::from(z);

        // 1. MACRO: Ridged Mountains with Pass Mask
        let ridge_val = self.macro_noise.get([x64 * MACRO_FREQ, z64 * MACRO_FREQ]);
        let ridge = (1.0 - ridge_val.abs()).powf(f64::from(MACRO_SHARPNESS)) as f32;

        let pass_val = self.pass_noise.get([x64 * PASS_FREQ, z64 * PASS_FREQ]);
        let pass_mask = ((pass_val + 1.0) * 0.5) as f32;

        let mountains = ridge * pass_mask * MACRO_HEIGHT;

        // 2. MICRO: Domain Warped Terraced Plateaus
        let wx = self.warp_noise_x.get([x64 * WARP_FREQ, z64 * WARP_FREQ]) as f32 * WARP_STRENGTH;
        let wz = self
            .warp_noise_z
            .get([x64 * WARP_FREQ + 100.0, z64 * WARP_FREQ + 100.0]) as f32
            * WARP_STRENGTH;

        let plateau_val = self.plateau_noise.get([
            (x64 + f64::from(wx)) * PLATEAU_FREQ,
            (z64 + f64::from(wz)) * PLATEAU_FREQ,
        ]);
        let plateau_base = ((plateau_val + 1.0) * 0.5) as f32;

        let plateaus = Self::smoothstep_terracing(plateau_base, PLATEAU_STEPS) * PLATEAU_HEIGHT;

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
        let gen = TerrainGenerator::new(123);
        let e1 = gen.get_elevation(0.0, 0.0);
        let e2 = gen.get_elevation(10.0, 10.0);

        // Check that it produces some value and is deterministic
        assert_eq!(e1, gen.get_elevation(0.0, 0.0));
        assert!(e1 >= 0.0);
        assert!(e2 >= 0.0);
    }
}
