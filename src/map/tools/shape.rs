use crate::game_state::{CurrentTool, EditorPhase, ShapeTool};
use crate::map::{HexCoord, MapData, RebuildMeshEvent, HEX_SIZE};
use bevy::prelude::*;

pub fn handle_shape_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut map_data: ResMut<MapData>,
    current_tool: Res<CurrentTool>,
    phase: Res<State<EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if *phase.get() != EditorPhase::Shape || current_tool.shape == ShapeTool::None {
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
                        let is_ocean = mouse.pressed(MouseButton::Left);
                        if tile.is_ocean != is_ocean {
                            tile.is_ocean = is_ocean;
                            if is_ocean {
                                tile.faction_id = None;
                            }
                            crate::map::validation::run_map_validation(&mut map_data);
                            ev_rebuild.write(RebuildMeshEvent);
                        }
                    }
                }
            }
        }
    }
}
