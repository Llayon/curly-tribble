use crate::map::data::OceanState;
use crate::map::{
    CliffLowerSide, EdgeCoord, EdgeData, EdgeType, HexCoord, LandscapeFeature, MapData,
};
use bevy::prelude::*;
use noise::{Fbm, NoiseFn, OpenSimplex};
use std::collections::HashMap;
use std::hash::BuildHasher;

pub struct CliffGenerationPlugin;

impl Plugin for CliffGenerationPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn generate_cliffs<S: BuildHasher>(
    map_data: &mut MapData,
    distance_field: &HashMap<HexCoord, u32, S>,
    seed: u32,
) {
    map_data.edges.clear();
    let plateau_noise = Fbm::<OpenSimplex>::new(seed + 60);

    let mut unique_edges = Vec::new();
    for &coord in map_data.tiles.keys() {
        for n in coord.neighbors() {
            if map_data.tiles.contains_key(&n) {
                unique_edges.push(EdgeCoord::new(coord, n));
            }
        }
    }
    unique_edges.sort_by_key(|e| (e.a, e.b));
    unique_edges.dedup();

    let mut new_cliffs = Vec::new();
    for edge in unique_edges {
        let Some(tile_a) = map_data.get_tile(edge.a.q, edge.a.r) else {
            continue;
        };
        let Some(tile_b) = map_data.get_tile(edge.b.q, edge.b.r) else {
            continue;
        };

        let feat_a = tile_a.landscape_feature;
        let feat_b = tile_b.landscape_feature;
        let is_high_a = feat_a == LandscapeFeature::Mountain || feat_a == LandscapeFeature::Plateau;
        let is_high_b = feat_b == LandscapeFeature::Mountain || feat_b == LandscapeFeature::Plateau;

        if (feat_a != feat_b) && (is_high_a || is_high_b) {
            let lower_side = if is_high_a && !is_high_b {
                CliffLowerSide::B
            } else if is_high_b && !is_high_a {
                CliffLowerSide::A
            } else {
                CliffLowerSide::Unresolved
            };
            new_cliffs.push((
                edge,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: lower_side,
                },
            ));
        } else if tile_a.ocean_state == OceanState::Land
            && tile_b.ocean_state == OceanState::Land
            && tile_a.faction_id.is_none()
            && tile_b.faction_id.is_none()
        {
            let d_a = *distance_field.get(&edge.a).unwrap_or(&0);
            let d_b = *distance_field.get(&edge.b).unwrap_or(&0);
            if d_a != d_b && (d_a.is_multiple_of(8) || d_b.is_multiple_of(8)) {
                let fault_noise =
                    plateau_noise.get([f64::from(edge.a.q) * 0.05, f64::from(edge.a.r) * 0.05]);
                if fault_noise > 0.3 {
                    let lower_side = match d_a.cmp(&d_b) {
                        std::cmp::Ordering::Less => CliffLowerSide::A,
                        std::cmp::Ordering::Greater => CliffLowerSide::B,
                        std::cmp::Ordering::Equal => CliffLowerSide::Unresolved,
                    };
                    new_cliffs.push((
                        edge,
                        EdgeData {
                            edge_type: EdgeType::Cliff,
                            cliff_lower_side: lower_side,
                        },
                    ));
                }
            }
        }
    }

    for (edge, data) in new_cliffs {
        map_data.edges.insert(edge, data);
    }
}
