use crate::game_state::EditorPhase;
use crate::map::treasures::{
    HiddenTreasure, MapToTarget, Targeting, TreasureDeposit, VisibleTreasure,
};
use bevy::prelude::*;

pub struct TreasureMeshPlugin;

impl Plugin for TreasureMeshPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Draws gizmos for treasures in the editor.
pub fn draw_treasure_gizmos(
    mut gizmos: Gizmos,
    q_treasures: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&VisibleTreasure>,
            Option<&HiddenTreasure>,
        ),
        With<TreasureDeposit>,
    >,
    q_links: Query<(&ChildOf, &Targeting), With<MapToTarget>>,
    phase: Res<State<EditorPhase>>,
) {
    if *phase.get() < EditorPhase::Treasures {
        return;
    }

    // 1. Draw Treasure Spheres
    for (_, transform, visible, hidden) in q_treasures.iter() {
        let color = if visible.is_some() {
            Color::srgb(0.0, 1.0, 0.5) // Emerald Green
        } else if hidden.is_some() {
            Color::srgb(1.0, 0.75, 0.0) // Amber Orange
        } else {
            Color::WHITE
        };

        gizmos.sphere(transform.translation() + Vec3::Y * 0.5, 0.4, color);
    }

    for (child_of, targeting) in q_links.iter() {
        let Ok((_, source_transform, _, _)) = q_treasures.get(child_of.0) else {
            continue;
        };
        let Ok((_, target_transform, _, _)) = q_treasures.get(targeting.target) else {
            continue;
        };

        gizmos.line(
            source_transform.translation() + Vec3::Y * 0.5,
            target_transform.translation() + Vec3::Y * 0.5,
            Color::srgb(0.0, 1.0, 0.0), // Green lines as per spec
        );
    }
}
