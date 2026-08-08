// src/map/surface_topology/twins.rs
//! Half-edge reciprocal twin construction for `SurfaceTopology`.

use crate::map::surface_topology::types::{
    SurfaceHalfEdgeId, SurfaceTopology, SurfaceTopologyError, SurfaceVertexId,
};
use bevy::prelude::*;
use std::collections::HashMap;

#[allow(dead_code)]
pub struct SurfaceTopologyTwinsPlugin;

impl Plugin for SurfaceTopologyTwinsPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Builds reciprocal twin half-edge references across all surface faces.
///
/// # Errors
/// Returns `SurfaceTopologyError` if an edge has non-manifold incidence (>2) or mismatched vertex directions.
#[allow(clippy::similar_names)]
pub fn build_half_edge_twins(surface: &mut SurfaceTopology) -> Result<(), SurfaceTopologyError> {
    let mut edge_buckets: HashMap<(usize, usize), Vec<SurfaceHalfEdgeId>> = HashMap::new();

    for (idx, he) in surface.half_edges.iter().enumerate() {
        let he_id = SurfaceHalfEdgeId::new(idx);
        let u0 = he.origin.index();
        let u1 = he.destination.index();
        let key = (u0.min(u1), u0.max(u1));
        edge_buckets.entry(key).or_default().push(he_id);
    }

    for ((u0_idx, u1_idx), bucket) in edge_buckets {
        match bucket.len() {
            1 => {
                // World/map boundary edge: twin remains None
            }
            2 => {
                let h_a_id = bucket[0];
                let h_b_id = bucket[1];
                let h_a = &surface.half_edges[h_a_id.index()];
                let h_b = &surface.half_edges[h_b_id.index()];

                if h_a.origin == h_b.destination && h_a.destination == h_b.origin {
                    surface.half_edges[h_a_id.index()].twin = Some(h_b_id);
                    surface.half_edges[h_b_id.index()].twin = Some(h_a_id);
                } else {
                    return Err(SurfaceTopologyError::TwinOrientationMismatch {
                        edge: h_a_id,
                        twin: h_b_id,
                    });
                }
            }
            count => {
                return Err(SurfaceTopologyError::NonManifoldEdge {
                    v0: SurfaceVertexId::new(u0_idx),
                    v1: SurfaceVertexId::new(u1_idx),
                    count,
                });
            }
        }
    }

    Ok(())
}
