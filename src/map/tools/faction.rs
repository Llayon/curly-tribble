use crate::game_state::{CurrentTool, EditorPhase, FactionManager, FactionTool};
use crate::map::{HexCoord, MapData, RebuildMeshEvent, HEX_SIZE};
use crate::map::tools::utils::get_mouse_world_pos;
use bevy::prelude::*;

pub fn handle_faction_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut map_data: ResMut<MapData>,
    current_tool: Res<CurrentTool>,
    faction_manager: Res<FactionManager>,
    phase: Res<State<EditorPhase>>,
    mut ev_rebuild: MessageWriter<RebuildMeshEvent>,
) {
    if *phase.get() != EditorPhase::Factions || current_tool.faction != FactionTool::Brush {
        return;
    }

    if mouse.pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) {
        if let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) {
            let coord = HexCoord::from_world(world_pos, HEX_SIZE);

            let target_faction = if mouse.pressed(MouseButton::Left) {
                faction_manager.selected_faction
            } else {
                None
            };

            if let Some(tile) = map_data.get_tile(coord.q, coord.r) {
                if tile.is_ocean {
                    return;
                }

                if let Some(existing_fid) = tile.faction_id {
                    if let Some(target_fid) = target_faction {
                        if existing_fid != target_fid {
                            return;
                        }
                    }
                }

                if let Some(f_id) = target_faction {
                    for neighbor_coord in coord.neighbors() {
                        if let Some(neighbor) =
                            map_data.get_tile(neighbor_coord.q, neighbor_coord.r)
                        {
                            if let Some(n_f_id) = neighbor.faction_id {
                                if n_f_id != f_id {
                                    return;
                                }
                            }
                        }
                    }
                }

                if let Some(tile_mut) = map_data.get_tile_mut(coord.q, coord.r) {
                    if tile_mut.faction_id == Some(1) {
                        return;
                    }

                    if tile_mut.faction_id != target_faction {
                        tile_mut.faction_id = target_faction;
                        ev_rebuild.write(RebuildMeshEvent);
                    }
                }
            }
        }
    }
}
