use crate::map::zoning::Roof;
use crate::map::MapData;
use crate::pawn::Pawn;
use bevy::prelude::*;

pub struct VisibilityPlugin;

impl Plugin for VisibilityPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            update_cave_visibility.in_set(crate::sets::GameSet::Visuals),
        );
    }
}

pub fn update_cave_visibility(
    pawns: Query<&Transform, With<Pawn>>,
    map: Res<MapData>,
    mut roofs: Query<&mut Visibility, With<Roof>>,
) {
    let mut any_pawn_in_cave = false;
    for transform in &pawns {
        let x = transform.translation.x.round() as i32;
        let z = transform.translation.z.round() as i32;
        if x >= 0 && z >= 0 {
            if let Some(tile) = map.get_tile(x as u32, z as u32) {
                if tile.roofed {
                    any_pawn_in_cave = true;
                    break;
                }
            }
        }
    }

    for mut vis in &mut roofs {
        *vis = if any_pawn_in_cave {
            Visibility::Hidden
        } else {
            Visibility::Inherited
        };
    }
}
