use crate::map::data::OceanState;
use crate::map::{
    EdgeCoord, EdgeData, EdgeDirection, EdgeType, HexCoord, LandscapeFeature, MapData,
};
use bevy::prelude::*;
use noise::{Fbm, NoiseFn, OpenSimplex};
use std::collections::HashMap;

pub struct CliffGenerationPlugin;

impl Plugin for CliffGenerationPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn generate_cliffs(map_data: &mut MapData, distance_field: &HashMap<HexCoord, u32>, seed: u32) {
    map_data.edges.clear();
    let plateau_noise = Fbm::<OpenSimplex>::new(seed + 60);
    let mut new_cliffs = Vec::new();

    let coords: Vec<_> = map_data.tiles.keys().copied().collect();
    for coord in coords {
        if let Some(tile_a) = map_data.get_tile(coord.q, coord.r) {
            let feat_a = tile_a.landscape_feature;
            for n in coord.neighbors() {
                if let Some(tile_b) = map_data.get_tile(n.q, n.r) {
                    let feat_b = tile_b.landscape_feature;
                    let mut is_cliff = false;
                    let mut direction = EdgeDirection::Normal;

                    if (feat_a != feat_b)
                        && (feat_a == LandscapeFeature::Mountain
                            || feat_a == LandscapeFeature::Plateau
                            || feat_b == LandscapeFeature::Mountain
                            || feat_b == LandscapeFeature::Plateau)
                    {
                        is_cliff = true;
                        direction = if feat_a == LandscapeFeature::Mountain
                            || feat_a == LandscapeFeature::Plateau
                        {
                            EdgeDirection::Normal
                        } else {
                            EdgeDirection::Reversed
                        };
                    } else if tile_a.ocean_state == OceanState::Land
                        && tile_b.ocean_state == OceanState::Land
                        && tile_a.faction_id.is_none()
                        && tile_b.faction_id.is_none()
                    {
                        let d_a = *distance_field.get(&coord).unwrap_or(&0) as i32;
                        let d_b = *distance_field.get(&n).unwrap_or(&0) as i32;
                        if d_a != d_b && (d_a % 12 == 0 || d_b % 12 == 0) {
                            let fault_noise = plateau_noise
                                .get([f64::from(coord.q) * 0.05, f64::from(coord.r) * 0.05]);
                            if fault_noise > 0.4 {
                                is_cliff = true;
                                direction = if d_a > d_b {
                                    EdgeDirection::Normal
                                } else {
                                    EdgeDirection::Reversed
                                };
                            }
                        }
                    }

                    if is_cliff {
                        let edge = EdgeCoord::new(coord, n);
                        new_cliffs.push((
                            edge,
                            EdgeData {
                                edge_type: EdgeType::Cliff,
                                direction,
                            },
                        ));
                    }
                }
            }
        }
    }

    for (edge, data) in new_cliffs {
        map_data.edges.insert(edge, data);
    }
}
