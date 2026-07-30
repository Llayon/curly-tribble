// src/map/tools/mine.rs
use crate::game_state::{CurrentTool, EditorPhase, MineTool};
use crate::map::mines::{MineBundle, MineDeposit};
use crate::map::tools::utils::get_mouse_world_pos;
use crate::map::{HexCoord, MapData, MapEntity, HEX_SIZE};
use bevy::prelude::*;

pub struct MineToolPlugin;

impl Plugin for MineToolPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn handle_mine_tools(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    current_tool: Res<CurrentTool>,
    phase: Res<State<EditorPhase>>,
    mut q_deposits: Query<(Entity, &mut MineDeposit), With<MineDeposit>>,
    map_data: Res<MapData>,
) {
    if *phase.get() != EditorPhase::Mines {
        return;
    }

    let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) else {
        return;
    };
    let center_coord = HexCoord::from_world(world_pos, HEX_SIZE);

    let is_paint = mouse.pressed(MouseButton::Left)
        || current_tool.mine_tool == MineTool::Paint && mouse.pressed(MouseButton::Left);
    let is_delete = mouse.pressed(MouseButton::Right)
        || current_tool.mine_tool == MineTool::Delete && mouse.pressed(MouseButton::Left);

    if is_paint {
        let Ok(brush_size) = i32::try_from(current_tool.mine_brush_size) else {
            return;
        };
        let mut target_coords = Vec::new();

        if brush_size <= 1 {
            target_coords.push(center_coord);
        } else {
            let n = brush_size - 1;
            for q in -n..=n {
                for r in ((-n).max(-q - n))..=((n).min(-q + n)) {
                    target_coords.push(HexCoord::new(center_coord.q + q, center_coord.r + r));
                }
            }
        }

        for coord in target_coords {
            // Check if hex already has a MineDeposit
            let mut existing_entity = None;
            for (entity, deposit) in q_deposits.iter() {
                if deposit.hex_coord == coord {
                    existing_entity = Some(entity);
                    break;
                }
            }

            if let Some(entity) = existing_entity {
                // Update existing deposit properties instead of spawning duplicate
                if let Ok((_, mut deposit)) = q_deposits.get_mut(entity) {
                    deposit.mine_type = current_tool.mine_type;
                    deposit.amount = current_tool.mine_amount;
                    deposit.depth = current_tool.mine_depth;
                }
            } else {
                // Check if hex is in map
                if let Some(_tile) = map_data.get_tile(coord.q, coord.r) {
                    let height = map_data.get_hex_height(coord.q, coord.r);
                    let mut pos = coord.to_world(HEX_SIZE);
                    pos.y = height;

                    commands.spawn(MineBundle {
                        deposit: MineDeposit {
                            mine_type: current_tool.mine_type,
                            amount: current_tool.mine_amount,
                            depth: current_tool.mine_depth,
                            hex_coord: coord,
                        },
                        name: Name::new(format!("{:?} Subsurface Mine", current_tool.mine_type)),
                        transform: Transform::from_translation(pos),
                        global_transform: GlobalTransform::default(),
                        visibility: Visibility::Visible,
                        inherited_visibility: InheritedVisibility::VISIBLE,
                        marker: MapEntity,
                    });
                }
            }
        }
    } else if is_delete {
        for (entity, deposit) in q_deposits.iter() {
            if deposit.hex_coord == center_coord {
                commands.entity(entity).despawn();
            }
        }
    }
}
