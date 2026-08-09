// src/map/height_graph/builder_diagnostics.rs
//! Diagnostic extraction and cycle detection for `HeightConstraintGraph`.

use crate::map::data::{CliffLowerSide, EdgeCoord};
use crate::map::height_graph::diagnostics::{
    HeightDiagnosticSeverity, HeightGraphDiagnostic, HeightGraphDiagnosticKind,
};
use crate::map::height_graph::types::{CliffNodeRelation, HeightNodeId};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

#[allow(dead_code)]
pub struct HeightGraphBuilderDiagnosticsPlugin;

impl Plugin for HeightGraphBuilderDiagnosticsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[must_use]
pub fn collect_height_graph_diagnostics(
    cliff_relations: &[CliffNodeRelation],
) -> Vec<HeightGraphDiagnostic> {
    let mut diagnostics = Vec::new();

    // 1. Info / Error: Collapsed cliff samples vs Unsplittable cliffs
    let mut cliff_groups: HashMap<EdgeCoord, Vec<&CliffNodeRelation>> = HashMap::new();
    for rel in cliff_relations {
        cliff_groups.entry(rel.logical_edge).or_default().push(rel);
    }

    for (edge, rels) in cliff_groups {
        let all_collapsed = !rels.is_empty() && rels.iter().all(|r| r.node_a == r.node_b);
        if all_collapsed {
            diagnostics.push(HeightGraphDiagnostic {
                severity: HeightDiagnosticSeverity::Error,
                kind: HeightGraphDiagnosticKind::UnsplittableCliff { edge },
            });
        } else {
            for rel in rels {
                if rel.node_a == rel.node_b {
                    diagnostics.push(HeightGraphDiagnostic {
                        severity: HeightDiagnosticSeverity::Info,
                        kind: HeightGraphDiagnosticKind::CollapsedCliffSample {
                            edge: rel.logical_edge,
                            vertex: rel.surface_vertex,
                        },
                    });
                }
            }
        }
    }

    // 2. Warning: Unresolved cliff lower side
    for rel in cliff_relations {
        if rel.lower_side == CliffLowerSide::Unresolved {
            diagnostics.push(HeightGraphDiagnostic {
                severity: HeightDiagnosticSeverity::Warning,
                kind: HeightGraphDiagnosticKind::UnresolvedCliff {
                    edge: rel.logical_edge,
                },
            });
        }
    }

    // 3. Build directed height ordering adjacency graph: lower -> higher
    let mut order_adj: HashMap<HeightNodeId, HashSet<HeightNodeId>> = HashMap::new();

    for rel in cliff_relations {
        if rel.node_a == rel.node_b {
            continue;
        }
        if let Some((lower, higher)) = rel.resolved_order() {
            order_adj.entry(lower).or_default().insert(higher);
        }
    }

    // 4. Check 2-node opposing ordering: lower -> higher AND higher -> lower
    let mut checked_pairs = HashSet::new();
    for (&u, neighbors) in &order_adj {
        for &v in neighbors {
            if u >= v {
                continue;
            }
            if let Some(rev_neighbors) = order_adj.get(&v) {
                let pair = (u, v);
                if rev_neighbors.contains(&u) && checked_pairs.insert(pair) {
                    diagnostics.push(HeightGraphDiagnostic {
                        severity: HeightDiagnosticSeverity::Error,
                        kind: HeightGraphDiagnosticKind::OpposedCliffOrdering { a: u, b: v },
                    });
                }
            }
        }
    }

    // 5. Check SCC cycles (>= 3 nodes) using Tarjan's algorithm
    let mut index = 0;
    let mut stack = Vec::new();
    let mut indices = HashMap::new();
    let mut lowlink = HashMap::new();
    let mut on_stack = HashSet::new();
    let mut sccs = Vec::new();

    let mut nodes_vec: Vec<HeightNodeId> = order_adj.keys().copied().collect();
    nodes_vec.sort();

    for node in nodes_vec {
        if !indices.contains_key(&node) {
            strongconnect(
                node,
                &order_adj,
                &mut index,
                &mut stack,
                &mut indices,
                &mut lowlink,
                &mut on_stack,
                &mut sccs,
            );
        }
    }

    for scc in sccs {
        if scc.len() >= 3 {
            let mut sorted_scc = scc;
            sorted_scc.sort();
            diagnostics.push(HeightGraphDiagnostic {
                severity: HeightDiagnosticSeverity::Error,
                kind: HeightGraphDiagnosticKind::DirectedCliffCycle {
                    component_nodes: sorted_scc,
                },
            });
        }
    }

    // 6. Canonical sort and dedup diagnostics
    diagnostics.sort();
    diagnostics.dedup();

    diagnostics
}

fn strongconnect(
    v: HeightNodeId,
    adj: &HashMap<HeightNodeId, HashSet<HeightNodeId>>,
    index: &mut usize,
    stack: &mut Vec<HeightNodeId>,
    indices: &mut HashMap<HeightNodeId, usize>,
    lowlink: &mut HashMap<HeightNodeId, usize>,
    on_stack: &mut HashSet<HeightNodeId>,
    sccs: &mut Vec<Vec<HeightNodeId>>,
) {
    indices.insert(v, *index);
    lowlink.insert(v, *index);
    *index += 1;
    stack.push(v);
    on_stack.insert(v);

    if let Some(neighbors) = adj.get(&v) {
        for &w in neighbors {
            if !indices.contains_key(&w) {
                strongconnect(w, adj, index, stack, indices, lowlink, on_stack, sccs);
                if let Some(&w_low) = lowlink.get(&w) {
                    if let Some(v_low) = lowlink.get_mut(&v) {
                        *v_low = (*v_low).min(w_low);
                    }
                }
            } else if on_stack.contains(&w) {
                if let Some(&w_index) = indices.get(&w) {
                    if let Some(v_low) = lowlink.get_mut(&v) {
                        *v_low = (*v_low).min(w_index);
                    }
                }
            }
        }
    }

    if let (Some(&low), Some(&idx)) = (lowlink.get(&v), indices.get(&v)) {
        if low == idx {
            let mut scc = Vec::new();
            while let Some(w) = stack.pop() {
                on_stack.remove(&w);
                scc.push(w);
                if w == v {
                    break;
                }
            }
            sccs.push(scc);
        }
    }
}
