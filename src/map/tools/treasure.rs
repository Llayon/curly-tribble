use crate::game_state::{CurrentTool, EditorPhase, TreasureToolMode};
use crate::map::tools::utils::get_mouse_world_pos;
use crate::map::{
    HexCoord, HiddenTreasure, LinkToolState, MapEntity, MapToTarget, Targeting, TreasureBundle,
    TreasureDeposit, TreasureItem, VisibleTreasure, HEX_SIZE,
};
use bevy::prelude::*;

pub struct TreasureToolPlugin;

impl Plugin for TreasureToolPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Bundle)]
pub struct MapToTargetBundle {
    pub name: Name,
    pub marker: MapToTarget,
    pub targeting: Targeting,
}

pub fn handle_treasure_tools(
    mut commands: Commands,
    phase: Res<State<EditorPhase>>,
    tool: Res<CurrentTool>,
    mut link_state: ResMut<LinkToolState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    q_deposits: Query<(Entity, &GlobalTransform), With<TreasureDeposit>>,
    mut q_deposit_mut: Query<&mut TreasureDeposit>,
    mut gizmos: Gizmos,
) {
    if *phase.get() != EditorPhase::Treasures {
        // Reset link state if we leave the phase
        if !matches!(*link_state, LinkToolState::Idle) {
            *link_state = LinkToolState::Idle;
        }
        return;
    }

    let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) else {
        return;
    };

    let hex_coord = HexCoord::from_world(world_pos, HEX_SIZE);
    let snapped_pos = hex_coord.to_world(HEX_SIZE);

    match tool.treasure_mode {
        TreasureToolMode::SpawnVisible | TreasureToolMode::SpawnHidden => {
            if mouse_input.just_pressed(MouseButton::Left) {
                let is_visible = tool.treasure_mode == TreasureToolMode::SpawnVisible;
                let entity = commands
                    .spawn(TreasureBundle {
                        deposit: TreasureDeposit {
                            contents: vec![],
                            hex_coord,
                        },
                        name: Name::new(if is_visible {
                            "Visible Treasure"
                        } else {
                            "Hidden Treasure"
                        }),
                        map_entity: MapEntity,
                        transform: Transform::from_translation(snapped_pos),
                        global_transform: GlobalTransform::default(),
                        visibility: Visibility::Visible,
                        inherited_visibility: InheritedVisibility::default(),
                    })
                    .id();

                if is_visible {
                    commands.entity(entity).insert(VisibleTreasure);
                } else {
                    commands.entity(entity).insert(HiddenTreasure);
                }
            }
        }
        TreasureToolMode::Link => {
            // Find deposit under mouse
            let mut hovered_deposit = None;
            for (entity, transform) in q_deposits.iter() {
                if transform.translation().distance(world_pos) < HEX_SIZE * 0.8 {
                    hovered_deposit = Some(entity);
                    break;
                }
            }

            match *link_state {
                LinkToolState::Idle => {
                    if mouse_input.just_pressed(MouseButton::Left) {
                        if let Some(entity) = hovered_deposit {
                            *link_state = LinkToolState::SelectingTarget(entity);
                        }
                    }
                }
                LinkToolState::SelectingTarget(source_entity) => {
                    // Visual link
                    let source_pos = if let Ok((_, transform)) = q_deposits.get(source_entity) {
                        transform.translation()
                    } else {
                        *link_state = LinkToolState::Idle;
                        return;
                    };

                    let mut line_color = Color::WHITE;

                    if let Some(target_entity) = hovered_deposit {
                        if target_entity != source_entity {
                            line_color = Color::srgb(0.0, 1.0, 0.0); // Green
                            if mouse_input.just_pressed(MouseButton::Left) {
                                if let Ok(mut source_deposit) = q_deposit_mut.get_mut(source_entity)
                                {
                                    source_deposit.contents.push(TreasureItem::TreasureMap);

                                    // Complies with Guard #18 (Semantic Graph) and #14 (Named Bundles):
                                    commands.entity(source_entity).with_children(|parent| {
                                        parent.spawn(MapToTargetBundle {
                                            name: Name::new("Map to Treasure"),
                                            marker: MapToTarget,
                                            targeting: Targeting {
                                                target: target_entity,
                                            },
                                        });
                                    });
                                }
                                *link_state = LinkToolState::Idle;
                            }
                        }
                    }

                    gizmos.line(
                        source_pos + Vec3::Y * 0.5,
                        world_pos + Vec3::Y * 0.5,
                        line_color,
                    );

                    if mouse_input.just_pressed(MouseButton::Right) {
                        *link_state = LinkToolState::Idle;
                    }
                }
            }
        }
    }

    // Delete treasures (Right Click in Spawn modes)
    if mouse_input.just_pressed(MouseButton::Right) && tool.treasure_mode != TreasureToolMode::Link
    {
        for (entity, transform) in q_deposits.iter() {
            if transform.translation().distance(world_pos) < HEX_SIZE * 0.8 {
                commands.entity(entity).despawn();
                break;
            }
        }
    }
}
