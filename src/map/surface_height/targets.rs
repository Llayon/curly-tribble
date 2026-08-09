// src/map/surface_height/targets.rs
//! Semantic preferred target and weight compilation for Milestone M5 — SurfaceHeightLayer.

use crate::map::height_constraints::types::RegionHeightIntent;
use crate::map::height_graph::types::{HeightConstraintGraph, HeightNodeId};
use crate::map::surface_height::guide::LegacyHeightGuide;
use crate::map::surface_height::types::HeightSolverConfig;
use crate::map::HexCoord;
use bevy::prelude::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeightTargetSample {
    pub target: f32,
    pub weight: f32,
}

#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct HeightTargetField {
    pub samples: Vec<HeightTargetSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeightTargetError {
    GuideCountMismatch,
    InvalidHeightNode(HeightNodeId),
    NonFiniteTarget(HeightNodeId),
    NonPositiveWeight(HeightNodeId),
}

/// Compiles semantic preferred target heights and effective anchor weights per node.
///
/// # Errors
/// Returns `HeightTargetError` if guide length mismatches node count or weights/targets are invalid.
#[allow(clippy::too_many_lines, clippy::type_complexity)]
pub fn compile_height_targets(
    graph: &HeightConstraintGraph,
    guide: &LegacyHeightGuide,
    config: &HeightSolverConfig,
) -> Result<HeightTargetField, HeightTargetError> {
    if guide.samples.len() != graph.nodes.len() {
        return Err(HeightTargetError::GuideCountMismatch);
    }

    // 1. Sort regions by hex (q, r) for canonical deterministic processing
    let mut sorted_regions: Vec<_> = graph.regions.iter().collect();
    sorted_regions.sort_by(|a, b| a.hex.q.cmp(&b.hex.q).then_with(|| a.hex.r.cmp(&b.hex.r)));

    // 2. Compute canonical region guide means and map region node contributions
    let mut region_means: BTreeMap<HexCoord, f32> = BTreeMap::new();
    let mut node_region_contribs: Vec<Vec<(HexCoord, RegionHeightIntent)>> =
        vec![Vec::new(); graph.nodes.len()];

    for &reg in &sorted_regions {
        let mut sum_g = 0.0f32;
        let count = reg.nodes.len();
        if count == 0 {
            continue;
        }

        for &node_id in &reg.nodes {
            let guide_sample = guide
                .samples
                .get(node_id.index())
                .ok_or(HeightTargetError::InvalidHeightNode(node_id))?;
            sum_g += guide_sample.target;
            node_region_contribs[node_id.index()].push((reg.hex, reg.intent));
        }

        region_means.insert(reg.hex, sum_g / (count as f32));
    }

    // 3. Compile preferred target and anchor weight for every HeightNodeId
    let mut samples = Vec::with_capacity(graph.nodes.len());

    for (node_idx, guide_sample) in guide.samples.iter().enumerate() {
        let node_id = HeightNodeId::new(node_idx);
        let mut weighted_sum = config.guide_weight * guide_sample.target;
        let mut total_weight = config.guide_weight;

        for &(reg_hex, intent) in &node_region_contribs[node_idx] {
            let reg_mean = *region_means.get(&reg_hex).unwrap_or(&guide_sample.target);
            let target_contrib = match intent {
                RegionHeightIntent::Mountain => guide_sample.target + config.mountain_bias,
                RegionHeightIntent::Plateau => reg_mean + config.plateau_bias,
                RegionHeightIntent::Lake => reg_mean + config.lake_bias,
                RegionHeightIntent::River => guide_sample.target + config.river_bias,
            };

            weighted_sum += config.region_weight * target_contrib;
            total_weight += config.region_weight;
        }

        if total_weight <= 0.0 || !total_weight.is_finite() {
            return Err(HeightTargetError::NonPositiveWeight(node_id));
        }

        let target = weighted_sum / total_weight;
        if !target.is_finite() {
            return Err(HeightTargetError::NonFiniteTarget(node_id));
        }

        samples.push(HeightTargetSample {
            target,
            weight: total_weight,
        });
    }

    Ok(HeightTargetField { samples })
}

#[allow(dead_code)]
pub struct HeightTargetsPlugin;

impl Plugin for HeightTargetsPlugin {
    fn build(&self, _app: &mut App) {}
}
