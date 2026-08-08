use crate::game_state::{CurrentTool, EditorPhase, LandscapeTool};
use crate::map::tools::utils::get_mouse_world_pos;
use crate::map::{
    HexCoord, LandscapeFeature, MapData, RebuildMeshEvent, HEX_SIZE,
};
use bevy::prelude::*;

pub struct LandscapeToolPlugin;

impl Plugin for LandscapeToolPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<super::landscape_edge_picker::LandscapeEdgePickIndex>();
    }
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
                LandscapeTool::Cliff | LandscapeTool::None => {}
            }
        }
    }
}
