use crate::map::HEX_SIZE;
use bevy::prelude::*;
use bevy_inspector_egui::prelude::*;
use noise::{Fbm, NoiseFn, OpenSimplex};

pub struct TerrainGenPlugin;

impl Plugin for TerrainGenPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Resource, Reflect, InspectorOptions, Clone)]
#[reflect(Resource, InspectorOptions)]
pub struct TerrainConfig {
    pub map_width: u32,
    pub map_height: u32,
    pub seed: u32,

    // --- INTEGRATED TOOLS (appear as checkboxes/buttons in inspector) ---
    pub randomize_seed: bool,
    pub regenerate_world: bool,

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

    #[inspector(min = 0.0, max = 2.0)]
    pub island_shape_weight: f32,

    #[inspector(min = 0, max = 30)]
    pub river_count: u32,
    #[inspector(min = 0.3, max = 0.9)]
    pub river_start_elevation: f32,
    #[inspector(min = 0.0, max = 0.2)]
    pub river_depth: f32,
    pub generate_mud_banks: bool,

    // --- VISUAL FILTERS ---
    pub show_build_area: bool,
    pub show_forests: bool,
    pub show_factions: bool,
    pub show_cliffs: bool,
}

impl Default for TerrainConfig {
    fn default() -> Self {
        Self {
            map_width: 40,
            map_height: 40,
            seed: 42,
            randomize_seed: false,
            regenerate_world: false,
            macro_freq: 0.04,
            macro_height: 12.0,
            macro_sharpness: 4.0,
            plateau_freq: 0.03,
            plateau_height: 5.0,
            plateau_steps: 3.0,
            warp_freq: 0.02,
            warp_strength: 2.0,
            island_shape_weight: 0.6,
            river_count: 8,
            river_start_elevation: 0.4,
            river_depth: 0.05,
            generate_mud_banks: true,
            show_build_area: false,
            show_forests: true,
            show_factions: true,
            show_cliffs: true,
        }
    }
}

#[derive(Resource)]
pub struct TerrainGenerator {
    macro_noise: Fbm<OpenSimplex>,
    plateau_noise: Fbm<OpenSimplex>,
    warp_noise_x: OpenSimplex,
    warp_noise_z: OpenSimplex,
}

impl TerrainGenerator {
    #[must_use]
    pub fn new(seed: u32) -> Self {
        Self {
            macro_noise: Fbm::<OpenSimplex>::new(seed),
            plateau_noise: Fbm::<OpenSimplex>::new(seed + 2),
            warp_noise_x: OpenSimplex::new(seed + 3),
            warp_noise_z: OpenSimplex::new(seed + 4),
        }
    }

    /// Lightweight 2D shape probability value [-1, 1].
    /// Used for Phase 1 (Shape) to determine land/ocean.
    #[must_use]
    pub fn get_shape_value(&self, config: &TerrainConfig, x: f32, z: f32) -> f32 {
        let x64 = f64::from(x);
        let z64 = f64::from(z);

        // DISTANCE MASK (MapGen4 Logic)
        let half_w = config.map_width as f32 * HEX_SIZE * 0.866;
        let half_h = config.map_height as f32 * HEX_SIZE * 0.75;
        let nx = (x / half_w).clamp(-1.5, 1.5);
        let nz = (z / half_h).clamp(-1.5, 1.5);
        let distance_sq = nx * nx + nz * nz;

        let island_shape = config.island_shape_weight * (0.75 - 2.0 * distance_sq);
        let ridge_val = self
            .macro_noise
            .get([x64 * config.macro_freq, z64 * config.macro_freq]) as f32;

        0.5 * (ridge_val + island_shape)
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation)] // Noise output f64 to f32 is intentional for mesh attributes
    pub fn get_elevation(&self, config: &TerrainConfig, x: f32, z: f32) -> f32 {
        let combined_shape = self.get_shape_value(config, x, z);

        if combined_shape <= 0.0 {
            return 0.0;
        }

        let x64 = f64::from(x);
        let z64 = f64::from(z);

        // MICRO: Domain Warped Terraced Plateaus
        let wx = self
            .warp_noise_x
            .get([x64 * config.warp_freq, z64 * config.warp_freq]) as f32
            * config.warp_strength;
        let wz = self.warp_noise_z.get([
            x64 * config.warp_freq + 100.0,
            z64 * config.warp_freq + 100.0,
        ]) as f32
            * config.warp_strength;

        let plateau_val = self.plateau_noise.get([
            (x64 + f64::from(wx)) * config.plateau_freq,
            (z64 + f64::from(wz)) * config.plateau_freq,
        ]);
        let plateau_base = ((plateau_val + 1.0) * 0.5) as f32;

        let plateaus =
            Self::smoothstep_terracing(plateau_base, config.plateau_steps) * config.plateau_height;

        // Apply our plateau details to the base shape
        (combined_shape * config.macro_height).max(plateaus * combined_shape.min(1.0))
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
