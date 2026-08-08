//! Integration and regression tests for warped water and roof overlays.

use bevy::prelude::*;

pub struct OverlayTestsPlugin;

impl Plugin for OverlayTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests_cases_advanced;
#[cfg(test)]
mod tests_cases_basic;
#[cfg(test)]
mod tests_matrix;
