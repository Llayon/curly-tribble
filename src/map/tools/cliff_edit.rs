// src/map/tools/cliff_edit.rs
//! Warped edge cliff stroke state and editing systems.

use crate::game_state::{CurrentTool, EditorPhase, LandscapeTool};
use crate::map::data::{CliffLowerSide, EdgeCoord, EdgeData, EdgeType, MapData};
use crate::map::face_topology::types::VertexId;
pub use crate::map::tools::cliff_picking::{
    classify_side, distance_sq_to_segment, pick_landscape_edge, LandscapeEdgeHit, LogicalEdgeSide,
};
use crate::map::tools::landscape_edge_picker::LandscapeEdgePickIndex;
use crate::map::tools::utils::get_mouse_world_pos;
use bevy::prelude::*;
use std::collections::HashSet;

#[allow(dead_code)]
pub struct CliffEditPlugin;

impl Plugin for CliffEditPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Resource, Default, Debug, Clone, PartialEq, Eq)]
pub struct HoveredCliffEdge {
    pub edge: Option<EdgeCoord>,
    pub side: Option<LogicalEdgeSide>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliffClickButton {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CliffStrokeMode {
    PaintUnresolved,
    OrientExisting,
    Erase,
}

#[derive(Resource, Default, Debug, Clone)]
pub struct CliffStrokeState {
    pub active: bool,
    pub mode: Option<CliffStrokeMode>,
    pub previous_accepted_edge: Option<EdgeCoord>,
    pub previous_accepted_vertices: Option<[VertexId; 2]>,
    pub visited_edges: HashSet<EdgeCoord>,
}

impl CliffStrokeState {
    pub fn reset(&mut self) {
        self.active = false;
        self.mode = None;
        self.previous_accepted_edge = None;
        self.previous_accepted_vertices = None;
        self.visited_edges.clear();
    }
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

#[allow(clippy::too_many_lines)]
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
            stroke_state.active = true;
            stroke_state.visited_edges.insert(hit.logical_edge);
            stroke_state.previous_accepted_edge = Some(hit.logical_edge);
            stroke_state.previous_accepted_vertices = Some(hit.vertices);

            if right_just_pressed {
                stroke_state.mode = Some(CliffStrokeMode::Erase);
                map_data.edges.remove(&hit.logical_edge);
            } else {
                let current_data = map_data
                    .edges
                    .get(&hit.logical_edge)
                    .copied()
                    .unwrap_or_default();
                if current_data.edge_type == EdgeType::Flat {
                    stroke_state.mode = Some(CliffStrokeMode::PaintUnresolved);
                    map_data.edges.insert(
                        hit.logical_edge,
                        EdgeData {
                            edge_type: EdgeType::Cliff,
                            cliff_lower_side: CliffLowerSide::Unresolved,
                        },
                    );
                } else {
                    stroke_state.mode = Some(CliffStrokeMode::OrientExisting);
                    if let Some(side) = hit.side {
                        let new_lower = match side {
                            LogicalEdgeSide::A => CliffLowerSide::A,
                            LogicalEdgeSide::B => CliffLowerSide::B,
                        };
                        map_data.edges.insert(
                            hit.logical_edge,
                            EdgeData {
                                edge_type: EdgeType::Cliff,
                                cliff_lower_side: new_lower,
                            },
                        );
                    }
                }
            }
        }
    } else if stroke_state.active && (left_pressed || right_pressed) {
        if let Some(ref hit) = current_hit {
            if !stroke_state.visited_edges.contains(&hit.logical_edge) {
                if let Some(prev_v) = stroke_state.previous_accepted_vertices {
                    let is_connected = hit.vertices[0] == prev_v[0]
                        || hit.vertices[0] == prev_v[1]
                        || hit.vertices[1] == prev_v[0]
                        || hit.vertices[1] == prev_v[1];

                    if is_connected {
                        stroke_state.visited_edges.insert(hit.logical_edge);
                        stroke_state.previous_accepted_edge = Some(hit.logical_edge);
                        stroke_state.previous_accepted_vertices = Some(hit.vertices);

                        match stroke_state.mode {
                            Some(CliffStrokeMode::Erase) => {
                                map_data.edges.remove(&hit.logical_edge);
                            }
                            Some(CliffStrokeMode::PaintUnresolved) => {
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
                                }
                            }
                            Some(CliffStrokeMode::OrientExisting) => {
                                let current_data = map_data
                                    .edges
                                    .get(&hit.logical_edge)
                                    .copied()
                                    .unwrap_or_default();
                                if current_data.edge_type == EdgeType::Cliff {
                                    if let Some(side) = hit.side {
                                        let new_lower = match side {
                                            LogicalEdgeSide::A => CliffLowerSide::A,
                                            LogicalEdgeSide::B => CliffLowerSide::B,
                                        };
                                        map_data.edges.insert(
                                            hit.logical_edge,
                                            EdgeData {
                                                edge_type: EdgeType::Cliff,
                                                cliff_lower_side: new_lower,
                                            },
                                        );
                                    }
                                }
                            }
                            None => {}
                        }
                    }
                }
            }
        }
    }
}
