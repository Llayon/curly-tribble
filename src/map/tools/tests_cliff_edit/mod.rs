// src/map/tools/tests_cliff_edit/mod.rs
//! Tests module for warped cliff editing.

use bevy::prelude::*;

#[allow(dead_code)]
pub struct CliffEditTestsPlugin;

impl Plugin for CliffEditTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests_matrix;

#[cfg(test)]
mod tests_picking;

#[cfg(test)]
mod tests_runtime;

#[cfg(test)]
mod tests_semantics;
