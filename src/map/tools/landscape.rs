use crate::game_state::{CurrentTool, EditorPhase, LandscapeTool};
use crate::map::tools::utils::get_mouse_world_pos;
use crate::map::{
    EdgeCoord, EdgeDirection, EdgeType, HexCoord, LandscapeFeature, MapData, RebuildMeshEvent,
    HEX_SIZE,
};
use bevy::prelude::*;

pub struct LandscapeToolPlugin;

impl Plugin for LandscapeToolPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn handle_landscape_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut map_data: ResMut<MapData>,
    current_tool: Res<CurrentTool>,
    phase: Res<State<EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if *phase.get() != EditorPhase::Landscape || current_tool.landscape == LandscapeTool::None {
        return;
    }

    if mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) {
        if let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) {
            let coord = HexCoord::from_world(world_pos, HEX_SIZE);

            match current_tool.landscape {
                LandscapeTool::Mountain
                | LandscapeTool::Lake
                | LandscapeTool::River
                | LandscapeTool::Plateau => {
                    if let Some(tile) = map_data.get_tile_mut(coord.q, coord.r) {
                        let new_feature = if mouse.pressed(MouseButton::Left) {
                            match current_tool.landscape {
                                LandscapeTool::Mountain => LandscapeFeature::Mountain,
                                LandscapeTool::Lake => LandscapeFeature::Lake,
                                LandscapeTool::River => LandscapeFeature::River,
                                LandscapeTool::Plateau => LandscapeFeature::Plateau,
                                _ => LandscapeFeature::None,
                            }
                        } else {
                            LandscapeFeature::None
                        };

                        if tile.landscape_feature != new_feature {
                            tile.landscape_feature = new_feature;
                            ev_rebuild.write(RebuildMeshEvent);
                        }
                    }
                }
                LandscapeTool::Cliff => {
                    let mut best_edge = None;
                    let mut min_dist = f32::MAX;

                    for neighbor in coord.neighbors() {
                        let edge = EdgeCoord::new(coord, neighbor);
                        let center_a = coord.to_world(HEX_SIZE);
                        let center_b = neighbor.to_world(HEX_SIZE);
                        let edge_midpoint = (center_a + center_b) * 0.5;

                        let dist = world_pos.distance(edge_midpoint);
                        if dist < min_dist && dist < HEX_SIZE * 0.6 {
                            min_dist = dist;
                            best_edge = Some(edge);
                        }
                    }

                    if let Some(edge) = best_edge {
                        if mouse.just_pressed(MouseButton::Left) {
                            let data = map_data.edges.get(&edge).copied().unwrap_or_default();
                            let mut new_data = data;
                            if data.edge_type == EdgeType::Flat {
                                new_data.edge_type = EdgeType::Cliff;
                                new_data.direction = EdgeDirection::Normal;
                            } else {
                                new_data.direction = if data.direction == EdgeDirection::Normal {
                                    EdgeDirection::Reversed
                                } else {
                                    EdgeDirection::Normal
                                };
                            }
                            map_data.edges.insert(edge, new_data);
                            ev_rebuild.write(RebuildMeshEvent);
                        } else if mouse.pressed(MouseButton::Right)
                            && map_data.edges.remove(&edge).is_some()
                        {
                            ev_rebuild.write(RebuildMeshEvent);
                        }
                    }
                }
                LandscapeTool::None => {}
            }
        }
    }
}
