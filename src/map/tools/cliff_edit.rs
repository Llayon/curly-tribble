// src/map/tools/cliff_edit.rs
//! Warped edge cliff stroke state and editing systems.

use crate::game_state::{CurrentTool, EditorPhase, LandscapeTool};
use crate::map::data::{CliffLowerSide, EdgeData, EdgeType, MapData};
pub use crate::map::tools::cliff_picking::{
    classify_side, distance_sq_to_segment, pick_landscape_edge, CliffClickButton, CliffStrokeMode,
    CliffStrokePhase, CliffStrokeState, HoveredCliffEdge, LandscapeEdgeHit, LogicalEdgeSide,
};
use crate::map::tools::landscape_edge_picker::LandscapeEdgePickIndex;
use crate::map::tools::utils::get_mouse_world_pos;
use bevy::prelude::*;

#[allow(dead_code)]
pub struct CliffEditPlugin;

impl Plugin for CliffEditPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn apply_single_cliff_click(
    map_data: &mut MapData,
    hit: &LandscapeEdgeHit,
    button: CliffClickButton,
) -> bool {
    if button == CliffClickButton::Secondary {
        return map_data.edges.remove(&hit.logical_edge).is_some();
    }

    if button == CliffClickButton::Primary {
        let current_data = map_data
            .edges
            .get(&hit.logical_edge)
            .copied()
            .unwrap_or_default();
        if current_data.edge_type == EdgeType::Flat {
            map_data.edges.insert(
                hit.logical_edge,
                EdgeData {
                    edge_type: EdgeType::Cliff,
                    cliff_lower_side: CliffLowerSide::Unresolved,
                },
            );
            return true;
        }

        if let Some(side) = hit.side {
            let new_lower = match side {
                LogicalEdgeSide::A => CliffLowerSide::A,
                LogicalEdgeSide::B => CliffLowerSide::B,
            };
            if current_data.cliff_lower_side != new_lower {
                map_data.edges.insert(
                    hit.logical_edge,
                    EdgeData {
                        edge_type: EdgeType::Cliff,
                        cliff_lower_side: new_lower,
                    },
                );
                return true;
            }
        }
    }

    false
}

#[allow(clippy::similar_names, clippy::too_many_lines)]
pub fn apply_cliff_stroke_step(
    map_data: &mut MapData,
    stroke_state: &mut CliffStrokeState,
    hit: &LandscapeEdgeHit,
    step_phase: CliffStrokePhase,
    left_click: bool,
    right_click: bool,
) -> bool {
    let current_data = map_data
        .edges
        .get(&hit.logical_edge)
        .copied()
        .unwrap_or_default();

    if step_phase == CliffStrokePhase::Initial {
        if right_click {
            if current_data.edge_type != EdgeType::Cliff {
                return false;
            }
            stroke_state.active = true;
            stroke_state.mode = Some(CliffStrokeMode::Erase);
            stroke_state.visited_edges.insert(hit.logical_edge);
            stroke_state.previous_accepted_edge = Some(hit.logical_edge);
            stroke_state.previous_accepted_vertices = Some(hit.vertices);
            map_data.edges.remove(&hit.logical_edge);
            return true;
        }

        if left_click {
            if current_data.edge_type == EdgeType::Flat {
                stroke_state.active = true;
                stroke_state.mode = Some(CliffStrokeMode::PaintUnresolved);
                stroke_state.visited_edges.insert(hit.logical_edge);
                stroke_state.previous_accepted_edge = Some(hit.logical_edge);
                stroke_state.previous_accepted_vertices = Some(hit.vertices);
                map_data.edges.insert(
                    hit.logical_edge,
                    EdgeData {
                        edge_type: EdgeType::Cliff,
                        cliff_lower_side: CliffLowerSide::Unresolved,
                    },
                );
                return true;
            }

            stroke_state.active = true;
            stroke_state.mode = Some(CliffStrokeMode::OrientExisting);
            stroke_state.visited_edges.insert(hit.logical_edge);
            stroke_state.previous_accepted_edge = Some(hit.logical_edge);
            stroke_state.previous_accepted_vertices = Some(hit.vertices);

            if let Some(side) = hit.side {
                let new_lower = match side {
                    LogicalEdgeSide::A => CliffLowerSide::A,
                    LogicalEdgeSide::B => CliffLowerSide::B,
                };
                if current_data.cliff_lower_side != new_lower {
                    map_data.edges.insert(
                        hit.logical_edge,
                        EdgeData {
                            edge_type: EdgeType::Cliff,
                            cliff_lower_side: new_lower,
                        },
                    );
                }
            }
            return true;
        }

        return false;
    }

    if !stroke_state.active || stroke_state.visited_edges.contains(&hit.logical_edge) {
        return false;
    }

    let Some(prev_v) = stroke_state.previous_accepted_vertices else {
        return false;
    };

    let is_connected = hit.vertices[0] == prev_v[0]
        || hit.vertices[0] == prev_v[1]
        || hit.vertices[1] == prev_v[0]
        || hit.vertices[1] == prev_v[1];

    if !is_connected {
        return false;
    }

    match stroke_state.mode {
        Some(CliffStrokeMode::Erase) => {
            if current_data.edge_type != EdgeType::Cliff {
                return false;
            }
            stroke_state.visited_edges.insert(hit.logical_edge);
            stroke_state.previous_accepted_edge = Some(hit.logical_edge);
            stroke_state.previous_accepted_vertices = Some(hit.vertices);
            map_data.edges.remove(&hit.logical_edge);
            true
        }
        Some(CliffStrokeMode::OrientExisting) => {
            if current_data.edge_type != EdgeType::Cliff {
                return false;
            }
            stroke_state.visited_edges.insert(hit.logical_edge);
            stroke_state.previous_accepted_edge = Some(hit.logical_edge);
            stroke_state.previous_accepted_vertices = Some(hit.vertices);

            if let Some(side) = hit.side {
                let new_lower = match side {
                    LogicalEdgeSide::A => CliffLowerSide::A,
                    LogicalEdgeSide::B => CliffLowerSide::B,
                };
                if current_data.cliff_lower_side != new_lower {
                    map_data.edges.insert(
                        hit.logical_edge,
                        EdgeData {
                            edge_type: EdgeType::Cliff,
                            cliff_lower_side: new_lower,
                        },
                    );
                }
            }
            true
        }
        Some(CliffStrokeMode::PaintUnresolved) => {
            stroke_state.visited_edges.insert(hit.logical_edge);
            stroke_state.previous_accepted_edge = Some(hit.logical_edge);
            stroke_state.previous_accepted_vertices = Some(hit.vertices);

            if current_data.edge_type == EdgeType::Flat {
                map_data.edges.insert(
                    hit.logical_edge,
                    EdgeData {
                        edge_type: EdgeType::Cliff,
                        cliff_lower_side: CliffLowerSide::Unresolved,
                    },
                );
            }
            true
        }
        None => false,
    }
}

pub fn handle_cliff_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    pick_index: Res<LandscapeEdgePickIndex>,
    mut map_data: ResMut<MapData>,
    current_tool: Res<CurrentTool>,
    phase: Res<State<EditorPhase>>,
    mut stroke_state: ResMut<CliffStrokeState>,
    mut hovered: ResMut<HoveredCliffEdge>,
) {
    if *phase.get() != EditorPhase::Landscape || current_tool.landscape != LandscapeTool::Cliff {
        if stroke_state.active {
            stroke_state.reset();
        }
        if hovered.edge.is_some() {
            *hovered = HoveredCliffEdge::default();
        }
        return;
    }

    let left_pressed = mouse.pressed(MouseButton::Left);
    let right_pressed = mouse.pressed(MouseButton::Right);

    if !left_pressed && !right_pressed && stroke_state.active {
        stroke_state.reset();
    }

    let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) else {
        *hovered = HoveredCliffEdge::default();
        return;
    };
    let cursor_xz = world_pos.xz();

    let current_hit = pick_landscape_edge(cursor_xz, &pick_index);

    if let Some(ref hit) = current_hit {
        hovered.edge = Some(hit.logical_edge);
        hovered.side = hit.side;
    } else {
        *hovered = HoveredCliffEdge::default();
    }

    let left_just_pressed = mouse.just_pressed(MouseButton::Left);
    let right_just_pressed = mouse.just_pressed(MouseButton::Right);

    if (left_just_pressed || right_just_pressed) && !stroke_state.active {
        if let Some(ref hit) = current_hit {
            apply_cliff_stroke_step(
                &mut map_data,
                &mut stroke_state,
                hit,
                CliffStrokePhase::Initial,
                left_just_pressed,
                right_just_pressed,
            );
        }
    } else if stroke_state.active && (left_pressed || right_pressed) {
        if let Some(ref hit) = current_hit {
            apply_cliff_stroke_step(
                &mut map_data,
                &mut stroke_state,
                hit,
                CliffStrokePhase::Subsequent,
                false,
                false,
            );
        }
    }
}
