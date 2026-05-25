use crate::game_state::{CurrentTool, EditorPhase, ShapeTool};
use crate::map::{HexCoord, MapData, RebuildMeshEvent, HEX_SIZE};
use crate::map::tools::utils::get_mouse_world_pos;
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
        if let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) {
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
