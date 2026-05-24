use crate::game_state::{CurrentTool, EditorPhase, LandscapeTool};
use crate::map::{EdgeCoord, HexCoord, LandscapeFeature, MapData, RebuildMeshEvent, HEX_SIZE};
use bevy::prelude::*;

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
        let Ok((camera, camera_transform)) = q_camera.single() else {
            return;
        };
        let Ok(window) = q_window.single() else {
            return;
        };

        if let Some(cursor_pos) = window.cursor_position() {
            if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_pos) {
                let distance = ray.origin.y / -ray.direction.y;
                if distance > 0.0 {
                    let world_pos = ray.origin + ray.direction * distance;
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
                                    let data =
                                        map_data.edges.get(&edge).copied().unwrap_or_default();
                                    let mut new_data = data;
                                    if !data.is_cliff {
                                        new_data.is_cliff = true;
                                        new_data.direction = true;
                                    } else {
                                        new_data.direction = !data.direction;
                                    }
                                    map_data.edges.insert(edge, new_data);
                                    ev_rebuild.write(RebuildMeshEvent);
                                } else if mouse.pressed(MouseButton::Right) {
                                    if map_data.edges.remove(&edge).is_some() {
                                        ev_rebuild.write(RebuildMeshEvent);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
