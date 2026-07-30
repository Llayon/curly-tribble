// src/map/mines.rs
use crate::game_state::{MineDepth, MineType};
use crate::map::{HexCoord, MapData, MapEntity, HEX_SIZE};
use bevy::prelude::*;
use rand::prelude::*;

#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct MineDeposit {
    pub mine_type: MineType,
    pub amount: u32,
    pub depth: MineDepth,
    pub hex_coord: HexCoord,
}

#[derive(Bundle)]
pub struct MineBundle {
    pub deposit: MineDeposit,
    pub name: Name,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub marker: MapEntity,
}

pub struct MinesPlugin;

impl Plugin for MinesPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<MineDeposit>();
    }
}

#[allow(clippy::too_many_lines)]
pub fn auto_spawn_mines(commands: &mut Commands, map_data: &MapData, seed: u32) {
    let mut rng = StdRng::seed_from_u64(u64::from(seed) + 200);
    let mut assigned: std::collections::HashMap<crate::map::HexCoord, (MineType, u32, MineDepth)> =
        std::collections::HashMap::new();

    for (coord, tile) in &map_data.tiles {
        if tile.ocean_state != crate::map::data::OceanState::Land || assigned.contains_key(coord) {
            continue;
        }

        let mut mine_type = None;
        let mut amount = 500;
        let mut depth = MineDepth::Shallow;

        match tile.landscape_feature {
            crate::map::LandscapeFeature::Mountain => {
                let roll = rng.gen_range(0.0..1.0);
                if roll < 0.4 {
                    mine_type = Some(MineType::Iron);
                    amount = rng.gen_range(600..1500);
                    depth = MineDepth::Medium;
                } else if roll < 0.7 {
                    mine_type = Some(MineType::Coal);
                    amount = rng.gen_range(800..2000);
                    depth = MineDepth::Shallow;
                } else if roll < 0.85 {
                    mine_type = Some(MineType::Gold);
                    amount = rng.gen_range(300..800);
                    depth = MineDepth::Deep;
                } else {
                    mine_type = Some(MineType::Stone);
                    amount = rng.gen_range(1000..2500);
                    depth = MineDepth::Shallow;
                }
            }
            crate::map::LandscapeFeature::Plateau => {
                let roll = rng.gen_range(0.0..1.0);
                if roll < 0.4 {
                    mine_type = Some(MineType::Copper);
                    amount = rng.gen_range(500..1200);
                    depth = MineDepth::Medium;
                } else if roll < 0.7 {
                    mine_type = Some(MineType::Coal);
                    amount = rng.gen_range(800..2000);
                    depth = MineDepth::Shallow;
                } else {
                    mine_type = Some(MineType::Stone);
                    amount = rng.gen_range(1000..2500);
                    depth = MineDepth::Shallow;
                }
            }
            _ => {
                if tile.terrain == crate::map::data::TerrainType::Stony && rng.gen_bool(0.15) {
                    mine_type = Some(MineType::Stone);
                    amount = rng.gen_range(500..1500);
                    depth = MineDepth::Shallow;
                }
            }
        }

        if let Some(m_type) = mine_type {
            let mut final_type = m_type;
            if matches!(m_type, MineType::Gold | MineType::Iron) {
                let mut clustered = false;
                for neighbor in coord.neighbors() {
                    if let Some(n_tile) = map_data.get_tile(neighbor.q, neighbor.r) {
                        if n_tile.ocean_state == crate::map::data::OceanState::Land
                            && !assigned.contains_key(&neighbor)
                            && (n_tile.landscape_feature == crate::map::LandscapeFeature::Mountain
                                || n_tile.landscape_feature
                                    == crate::map::LandscapeFeature::Plateau)
                        {
                            assigned.insert(neighbor, (m_type, amount, depth));
                            clustered = true;
                            break;
                        }
                    }
                }
                if !clustered {
                    final_type = if m_type == MineType::Iron {
                        MineType::Coal
                    } else {
                        MineType::Stone
                    };
                }
            }
            assigned.insert(*coord, (final_type, amount, depth));
        }
    }

    for (coord, (m_type, amount, depth)) in assigned {
        let world_pos = coord.to_world(HEX_SIZE);
        commands.spawn(MineBundle {
            deposit: MineDeposit {
                mine_type: m_type,
                amount,
                depth,
                hex_coord: coord,
            },
            name: Name::new(format!("{m_type:?} Mine")),
            transform: Transform::from_translation(world_pos),
            global_transform: GlobalTransform::default(),
            visibility: Visibility::Visible,
            inherited_visibility: InheritedVisibility::default(),
            marker: MapEntity,
        });
    }
}
