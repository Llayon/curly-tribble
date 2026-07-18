use crate::game_state::{ArtifactToolState, EditorPhase};
use crate::map::tools::utils::get_mouse_world_pos;
use crate::map::{Artifact, ArtifactLocation, HexCoord, HEX_SIZE};
use bevy::prelude::*;

pub struct ArtifactToolPlugin;

impl Plugin for ArtifactToolPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn handle_artifact_tools(
    mut commands: Commands,
    phase: Res<State<EditorPhase>>,
    mut state: ResMut<ArtifactToolState>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    q_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    q_window: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut q_artifact: Query<(&mut Artifact, &mut Transform), With<Artifact>>,
) {
    if *phase.get() != EditorPhase::Artifacts || !state.placing_on_ground {
        return;
    }

    let Some(selected_entity) = state.selected_artifact else {
        return;
    };

    let Some(world_pos) = get_mouse_world_pos(&q_camera, &q_window) else {
        return;
    };

    let hex_coord = HexCoord::from_world(world_pos, HEX_SIZE);
    let snapped_pos = hex_coord.to_world(HEX_SIZE);

    if mouse_input.just_pressed(MouseButton::Left) {
        if let Ok((mut artifact, mut transform)) = q_artifact.get_mut(selected_entity) {
            artifact.location = ArtifactLocation::OnGround(hex_coord);
            transform.translation = snapped_pos;
            commands
                .entity(selected_entity)
                .remove::<crate::map::StoredInTreasure>();
        }
        state.placing_on_ground = false;
    }

    if mouse_input.just_pressed(MouseButton::Right) {
        state.placing_on_ground = false;
    }
}
