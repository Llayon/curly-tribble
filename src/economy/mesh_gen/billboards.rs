// src/economy/mesh_gen/billboards.rs
use crate::map::deposits::{DepositType, ResourceDeposit};
use crate::map::terrain_gen::TerrainConfig;
use crate::map::{MapData, HEX_SIZE};
use bevy::prelude::*;

use crate::game_state::EditorPhase;

pub struct BillboardPlugin;

impl Plugin for BillboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            draw_bio_billboards
                .run_if(|phase: Res<State<EditorPhase>>| *phase.get() >= EditorPhase::Plants),
        );
    }
}

pub fn draw_bio_billboards(
    mut gizmos: Gizmos,
    query: Query<&ResourceDeposit>,
    map_data: Res<MapData>,
    config: Res<TerrainConfig>,
) {
    if !config.show_forests {
        return;
    }

    for deposit in query.iter() {
        let world_pos_2d = deposit.hex_coord.to_world(HEX_SIZE);
        let height = map_data.get_hex_height(deposit.hex_coord.q, deposit.hex_coord.r);
        let pos = Vec3::new(world_pos_2d.x, height + 0.5, world_pos_2d.z);

        let color = match deposit.deposit_type {
            DepositType::WildFlax
            | DepositType::Raspberries
            | DepositType::Pumpkin
            | DepositType::WildWheat => {
                Color::srgb(0.2, 0.8, 0.2) // Green for plants
            }
            DepositType::Rabbit | DepositType::Deer | DepositType::Boar => {
                Color::srgb(0.6, 0.4, 0.2) // Brown for land animals
            }
            DepositType::OceanFish => {
                Color::srgb(0.2, 0.4, 1.0) // Blue for fish
            }
        };

        let mut final_color = color;
        if !deposit.habitat_valid {
            final_color = Color::srgb(1.0, 0.2, 0.2); // Red tint for invalid habitat
        }

        // Draw a circle-like icon (billboard-ish)
        // In Bevy 0.18.1 Gizmos::circle takes (position, radius, color)
        gizmos.circle(pos, 0.4, final_color);

        // Add a sphere for more "3D" presence
        gizmos.sphere(pos, 0.2, final_color);

        if !deposit.habitat_valid {
            // Draw a red "X" or something to indicate error
            gizmos.line(
                pos + Vec3::new(-0.3, 0.3, 0.0),
                pos + Vec3::new(0.3, -0.3, 0.0),
                Color::WHITE,
            );
            gizmos.line(
                pos + Vec3::new(0.3, 0.3, 0.0),
                pos + Vec3::new(-0.3, -0.3, 0.0),
                Color::WHITE,
            );
        }
    }
}
