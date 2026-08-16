// src/map/surface_gameplay/config.rs
//! Milestone M6 — `SurfaceGameplayConfig`: policy thresholds and movement
//! costs, plus a stable fingerprint for change detection.

use bevy::prelude::*;

pub struct SurfaceGameplayConfigPlugin;

impl Plugin for SurfaceGameplayConfigPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Policy thresholds are NORMALIZED (`[0, 1]`, M5 height domain).
/// Movement costs are legacy-compatible: 20 base / 50 swamp / 80 stony.
#[derive(Resource, Debug, Clone, PartialEq, Reflect)]
#[reflect(Resource)]
pub struct SurfaceGameplayConfig {
    /// Max absolute height delta (normalized) that a foot agent can climb.
    pub max_walk_step: f32,
    /// Max cell relief (normalized) that permits construction.
    pub max_build_relief: f32,
    /// Max neighbor height delta (normalized) that permits construction.
    pub max_build_neighbor_step: f32,
    /// Movement cost of ordinary land cells.
    pub walk_base_cost: u8,
    /// Movement cost of swamp cells.
    pub swamp_cost: u8,
    /// Movement cost of stony cells.
    pub stony_cost: u8,
}

impl Default for SurfaceGameplayConfig {
    fn default() -> Self {
        Self {
            max_walk_step: 0.30,
            max_build_relief: 0.30,
            max_build_neighbor_step: 0.30,
            walk_base_cost: 20,
            swamp_cost: 50,
            stony_cost: 80,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceGameplayConfigError {
    NonFiniteValue,
    InvalidMaxWalkStep,
    InvalidMaxBuildRelief,
    InvalidMaxBuildNeighborStep,
    BaseCostNotPositive,
    SwampCostNotGreaterThanBase,
    StonyCostNotGreaterThanBase,
}

impl SurfaceGameplayConfig {
    /// Validates finiteness, normalized ranges, and cost ordering.
    ///
    /// # Errors
    /// Returns `SurfaceGameplayConfigError` on any invalid field.
    pub fn validate_config(&self) -> Result<(), SurfaceGameplayConfigError> {
        for f in [
            self.max_walk_step,
            self.max_build_relief,
            self.max_build_neighbor_step,
        ] {
            if !f.is_finite() {
                return Err(SurfaceGameplayConfigError::NonFiniteValue);
            }
        }

        if !(0.0..=1.0).contains(&self.max_walk_step) {
            return Err(SurfaceGameplayConfigError::InvalidMaxWalkStep);
        }
        if !(0.0..=1.0).contains(&self.max_build_relief) {
            return Err(SurfaceGameplayConfigError::InvalidMaxBuildRelief);
        }
        if !(0.0..=1.0).contains(&self.max_build_neighbor_step) {
            return Err(SurfaceGameplayConfigError::InvalidMaxBuildNeighborStep);
        }

        if self.walk_base_cost == 0 {
            return Err(SurfaceGameplayConfigError::BaseCostNotPositive);
        }
        if self.swamp_cost <= self.walk_base_cost {
            return Err(SurfaceGameplayConfigError::SwampCostNotGreaterThanBase);
        }
        if self.stony_cost <= self.walk_base_cost {
            return Err(SurfaceGameplayConfigError::StonyCostNotGreaterThanBase);
        }

        Ok(())
    }
}

/// Stable fingerprint of every policy-affecting field (bit-exact floats).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceGameplayConfigFingerprint {
    pub max_walk_step_bits: u32,
    pub max_build_relief_bits: u32,
    pub max_build_neighbor_step_bits: u32,
    pub walk_base_cost: u8,
    pub swamp_cost: u8,
    pub stony_cost: u8,
}

impl SurfaceGameplayConfigFingerprint {
    #[must_use]
    pub fn from_config(c: &SurfaceGameplayConfig) -> Self {
        Self {
            max_walk_step_bits: c.max_walk_step.to_bits(),
            max_build_relief_bits: c.max_build_relief.to_bits(),
            max_build_neighbor_step_bits: c.max_build_neighbor_step.to_bits(),
            walk_base_cost: c.walk_base_cost,
            swamp_cost: c.swamp_cost,
            stony_cost: c.stony_cost,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = SurfaceGameplayConfig::default();
        assert!(config.validate_config().is_ok());
    }

    #[test]
    fn reject_non_finite_threshold() {
        let mut config = SurfaceGameplayConfig::default();
        config.max_walk_step = f32::NAN;
        assert_eq!(
            config.validate_config(),
            Err(SurfaceGameplayConfigError::NonFiniteValue)
        );
    }

    #[test]
    fn reject_out_of_range_thresholds() {
        let mut config = SurfaceGameplayConfig::default();
        config.max_build_relief = 1.5;
        assert_eq!(
            config.validate_config(),
            Err(SurfaceGameplayConfigError::InvalidMaxBuildRelief)
        );

        let mut config = SurfaceGameplayConfig::default();
        config.max_build_neighbor_step = -0.1;
        assert_eq!(
            config.validate_config(),
            Err(SurfaceGameplayConfigError::InvalidMaxBuildNeighborStep)
        );
    }

    #[test]
    fn reject_zero_base_cost() {
        let mut config = SurfaceGameplayConfig::default();
        config.walk_base_cost = 0;
        assert_eq!(
            config.validate_config(),
            Err(SurfaceGameplayConfigError::BaseCostNotPositive)
        );
    }

    #[test]
    fn reject_non_ascending_special_costs() {
        let mut config = SurfaceGameplayConfig::default();
        config.swamp_cost = config.walk_base_cost;
        assert_eq!(
            config.validate_config(),
            Err(SurfaceGameplayConfigError::SwampCostNotGreaterThanBase)
        );

        let mut config = SurfaceGameplayConfig::default();
        config.stony_cost = config.walk_base_cost - 1;
        assert_eq!(
            config.validate_config(),
            Err(SurfaceGameplayConfigError::StonyCostNotGreaterThanBase)
        );
    }

    #[test]
    fn fingerprint_matches_default_and_tracks_all_fields() {
        let default = SurfaceGameplayConfig::default();
        let fp_default = SurfaceGameplayConfigFingerprint::from_config(&default);

        let mut tweaked = default.clone();
        tweaked.max_walk_step += 0.001;
        let fp_tweaked = SurfaceGameplayConfigFingerprint::from_config(&tweaked);
        assert_ne!(fp_default, fp_tweaked);

        let mut tweaked = default.clone();
        tweaked.stony_cost = 99;
        let fp_tweaked = SurfaceGameplayConfigFingerprint::from_config(&tweaked);
        assert_ne!(fp_default, fp_tweaked);

        assert_eq!(fp_default, fp_default);
    }
}
