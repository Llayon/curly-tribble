use crate::game_state::{CurrentTool, EditorPhase};
use crate::map::{ForestType, HexCoord, MapData, RebuildMeshEvent, TerrainType, HEX_SIZE};
use bevy::prelude::*;

pub fn handle_sediment_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut map_data: ResMut<MapData>,
    current_tool: Res<CurrentTool>,
    phase: Res<State<EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if *phase.get() != EditorPhase::Sediments {
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

                    if let Some(tile) = map_data.get_tile_mut(coord.q, coord.r) {
                        let mut changed = false;

                        if current_tool.active_sediment_tool {
                            let target_sediment = if mouse.pressed(MouseButton::Left) {
                                current_tool.sediment
                            } else {
                                TerrainType::Grass
                            };

                            if tile.terrain != target_sediment {
                                tile.terrain = target_sediment;
                                changed = true;
                            }
                        }

                        if current_tool.active_forest_tool {
                            let (target_type, target_density) = if mouse.pressed(MouseButton::Left)
                            {
                                (current_tool.forest_type, current_tool.forest_density)
                            } else {
                                (ForestType::None, 0.0)
                            };

                            if tile.forest_type != target_type
                                || (tile.forest_density - target_density).abs() > 0.01
                            {
                                tile.forest_type = target_type;
                                tile.forest_density = target_density;
                                changed = true;
                            }
                        }

                        if changed {
                            ev_rebuild.write(RebuildMeshEvent);
                        }
                    }
                }
            }
        }
    }
}
