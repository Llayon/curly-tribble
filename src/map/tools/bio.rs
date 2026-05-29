use crate::game_state::{CurrentTool, EditorPhase};
use crate::map::tools::utils::get_mouse_world_pos;
use crate::map::{HexCoord, MapData, MapEntity, ResourceDeposit, ResourceDepositBundle, HEX_SIZE};
use bevy::prelude::*;

pub struct BioToolPlugin;

impl Plugin for BioToolPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn handle_bio_tools(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    current_tool: Res<CurrentTool>,
    phase: Res<State<EditorPhase>>,
    q_deposits: Query<(Entity, &ResourceDeposit)>,
    map_data: Res<MapData>,
) {
    if *phase.get() != EditorPhase::Plants {
        return;
    }

    let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) else {
        return;
    };
    let center_coord = HexCoord::from_world(world_pos, HEX_SIZE);

    if mouse.pressed(MouseButton::Left) {
        let brush_size = current_tool.bio_brush_size as i32;
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
            // Check if hex already has a ResourceDeposit
            let mut already_exists = false;
            for (_, deposit) in q_deposits.iter() {
                if deposit.hex_coord == coord {
                    already_exists = true;
                    break;
                }
            }

            if !already_exists {
                // Check if hex is in map
                if let Some(_tile) = map_data.get_tile(coord.q, coord.r) {
                    let height = map_data.get_hex_height(coord.q, coord.r);
                    let mut pos = coord.to_world(HEX_SIZE);
                    pos.y = height;

                    commands.spawn(ResourceDepositBundle {
                        deposit: ResourceDeposit {
                            deposit_type: current_tool.bio_resource,
                            amount: current_tool.bio_amount,
                            hex_coord: coord,
                            habitat_valid: true,
                        },
                        name: Name::new(format!("{:?}", current_tool.bio_resource)),
                        transform: Transform::from_translation(pos),
                        visibility: Visibility::Visible,
                        inherited_visibility: InheritedVisibility::VISIBLE,
                        marker: MapEntity,
                    });
                }
            }
        }
    } else if mouse.pressed(MouseButton::Right) {
        for (entity, deposit) in q_deposits.iter() {
            if deposit.hex_coord == center_coord {
                commands.entity(entity).despawn();
            }
        }
    }
}
