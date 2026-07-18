use crate::game_state::{CurrentTool, EditorPhase, FactionManager, NpcTool};
use crate::map::data::OceanState;
use crate::map::tools::utils::get_mouse_world_pos;
use crate::map::{
    EnemyCamp, EnemyCampBundle, HexCoord, MapData, MapEntity, PoiBundle, PointOfInterest, HEX_SIZE,
};
use bevy::prelude::*;

pub struct NpcToolPlugin;

impl Plugin for NpcToolPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn handle_npc_tools(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    map_data: Res<MapData>,
    current_tool: Res<CurrentTool>,
    faction_manager: Res<FactionManager>,
    phase: Res<State<EditorPhase>>,
    q_pois: Query<(Entity, &PointOfInterest)>,
    q_camps: Query<(Entity, &EnemyCamp)>,
) {
    if *phase.get() != EditorPhase::NPCs || current_tool.npc == NpcTool::None {
        return;
    }

    if mouse.just_pressed(MouseButton::Left) || mouse.pressed(MouseButton::Right) {
        if let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) {
            let coord = HexCoord::from_world(world_pos, HEX_SIZE);

            if let Some(tile) = map_data.get_tile(coord.q, coord.r) {
                if tile.ocean_state == OceanState::Ocean {
                    return;
                }

                let faction = faction_manager
                    .factions
                    .iter()
                    .find(|f| Some(f.id) == tile.faction_id);
                let f_type = faction.map(|f| f.faction_type);

                match current_tool.npc {
                    NpcTool::SpawnPoi => {
                        if f_type == Some(crate::game_state::FactionType::Enemy) {
                            return;
                        }
                        if mouse.just_pressed(MouseButton::Left) {
                            for (e, poi) in &q_pois {
                                if poi.hex_coord == coord {
                                    commands.entity(e).despawn();
                                }
                            }

                            commands.spawn(PoiBundle {
                                poi: PointOfInterest {
                                    poi_type: current_tool.poi_type,
                                    hex_coord: coord,
                                    linked_objective_id: None,
                                },
                                name: Name::new(format!(
                                    "{:?} at {:?}",
                                    current_tool.poi_type, coord
                                )),
                                transform: Transform::from_translation(coord.to_world(HEX_SIZE)),
                                visibility: Visibility::Visible,
                                inherited_visibility: InheritedVisibility::default(),
                                marker: MapEntity,
                            });
                        }
                    }
                    NpcTool::SpawnEnemyCamp => {
                        if f_type == Some(crate::game_state::FactionType::Player)
                            || f_type == Some(crate::game_state::FactionType::Neutral)
                        {
                            return;
                        }
                        if mouse.just_pressed(MouseButton::Left) {
                            for (e, camp) in &q_camps {
                                if camp.hex_coord == coord {
                                    commands.entity(e).despawn();
                                }
                            }

                            commands.spawn(EnemyCampBundle {
                                camp: EnemyCamp {
                                    hex_coord: coord,
                                    sub_faction: "Bandits".to_string(),
                                    difficulty: current_tool.camp_difficulty,
                                    combat_power: current_tool.camp_power,
                                    camp_count: 1,
                                },
                                name: Name::new(format!("Enemy Camp at {coord:?}")),
                                transform: Transform::from_translation(coord.to_world(HEX_SIZE)),
                                visibility: Visibility::Visible,
                                inherited_visibility: InheritedVisibility::default(),
                                marker: MapEntity,
                            });
                        }
                    }
                    NpcTool::Delete => {
                        for (e, poi) in &q_pois {
                            if poi.hex_coord == coord {
                                commands.entity(e).despawn();
                            }
                        }
                        for (e, camp) in &q_camps {
                            if camp.hex_coord == coord {
                                commands.entity(e).despawn();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
