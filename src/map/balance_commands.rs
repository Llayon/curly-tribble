// src/map/balance_commands.rs
use crate::economy::assets::GameAssets;
use crate::map::balance::StarterResource;
use crate::map::data::OceanState;
use crate::map::deposits::{DepositType, ResourceDeposit, ResourceDepositBundle};
use crate::map::resources::{BerryBush, BerryBushBundle};
use crate::map::{
    ForestType, HexCoord, LandscapeFeature, MapData, MapEntity, RebuildMeshEvent, TerrainType,
    HEX_SIZE,
};
use bevy::prelude::*;

pub struct BalanceCommandsPlugin;

impl Plugin for BalanceCommandsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub struct AutoBalanceCommand {
    pub faction_id: u32,
}

impl Command for AutoBalanceCommand {
    #[allow(clippy::too_many_lines)]
    fn apply(self, world: &mut World) {
        let has_faction = world.get_resource::<MapData>().is_some_and(|m| {
            m.tiles
                .values()
                .any(|t| t.faction_id == Some(self.faction_id))
        });
        if !has_faction {
            let seed_val = world
                .get_resource::<crate::map::WorldSeed>()
                .map_or(12345, crate::map::WorldSeed::value);
            if let Some(mut m) = world.get_resource_mut::<MapData>() {
                crate::map::generation::auto_spawn_player_territory(&mut m, seed_val);
            }
        }

        let deficiencies = {
            let map_data = if let Some(m) = world.get_resource::<MapData>() {
                m.clone()
            } else {
                return;
            };

            let mut wood_count = 0;
            for tile in map_data.tiles.values() {
                if tile.faction_id == Some(self.faction_id)
                    && tile.forest_type != ForestType::None
                    && tile.forest_density > 0.1
                {
                    wood_count += 1;
                }
            }

            let mut has_food = false;
            let mut has_flax = false;

            let mut q_dep = world.query::<&ResourceDeposit>();
            for deposit in q_dep.iter(world) {
                if let Some(tile) = map_data.tiles.get(&deposit.hex_coord) {
                    if tile.faction_id == Some(self.faction_id) {
                        if matches!(
                            deposit.deposit_type,
                            DepositType::Raspberries
                                | DepositType::Rabbit
                                | DepositType::Deer
                                | DepositType::Boar
                                | DepositType::Pumpkin
                                | DepositType::WildWheat
                        ) {
                            has_food = true;
                        }
                        if deposit.deposit_type == DepositType::WildFlax {
                            has_flax = true;
                        }
                    }
                }
            }

            if !has_food {
                let mut q_bush = world.query_filtered::<&Transform, With<BerryBush>>();
                for transform in q_bush.iter(world) {
                    let coord = HexCoord::from_world(transform.translation, HEX_SIZE);
                    if let Some(tile) = map_data.tiles.get(&coord) {
                        if tile.faction_id == Some(self.faction_id) {
                            has_food = true;
                            break;
                        }
                    }
                }
            }

            let mut def = Vec::new();
            if wood_count < 5 {
                def.push((StarterResource::Wood, 5 - wood_count));
            }
            if !has_food {
                def.push((StarterResource::Food, 1));
            }
            if !has_flax {
                def.push((StarterResource::Flax, 1));
            }
            def
        };

        if deficiencies.is_empty() {
            return;
        }

        let mut occupied = std::collections::HashSet::new();

        let mut q_dep = world.query::<&ResourceDeposit>();
        for dep in q_dep.iter(world) {
            occupied.insert(dep.hex_coord);
        }

        let mut q_bush = world.query_filtered::<&Transform, With<BerryBush>>();
        for transform in q_bush.iter(world) {
            let coord = HexCoord::from_world(transform.translation, HEX_SIZE);
            occupied.insert(coord);
        }

        let mut q_poi = world.query::<&crate::map::poi::PointOfInterest>();
        for poi in q_poi.iter(world) {
            occupied.insert(poi.hex_coord);
        }

        let mut q_camp = world.query::<&crate::map::camps::EnemyCamp>();
        for camp in q_camp.iter(world) {
            occupied.insert(camp.hex_coord);
        }

        let mut q_mine = world.query::<&crate::map::mines::MineDeposit>();
        for mine in q_mine.iter(world) {
            occupied.insert(mine.hex_coord);
        }

        let mut q_treasure = world.query::<&crate::map::treasures::TreasureDeposit>();
        for treasure in q_treasure.iter(world) {
            occupied.insert(treasure.hex_coord);
        }

        let mut q_artifact = world.query::<&crate::map::artifacts::Artifact>();
        for art in q_artifact.iter(world) {
            if let crate::map::artifacts::ArtifactLocation::OnGround(c) = art.location {
                occupied.insert(c);
            }
        }

        let mut vacant_coords = Vec::new();
        if let Some(map_data) = world.get_resource::<MapData>() {
            for (coord, tile) in &map_data.tiles {
                if tile.faction_id == Some(self.faction_id)
                    && tile.ocean_state == OceanState::Land
                    && tile.landscape_feature == LandscapeFeature::None
                    && !occupied.contains(coord)
                {
                    vacant_coords.push(*coord);
                }
            }
        }
        vacant_coords.sort_by_key(|c| (c.q, c.r));

        let mut vacant_iter = vacant_coords.into_iter();

        for (def_type, count) in deficiencies {
            match def_type {
                StarterResource::Wood => {
                    for _ in 0..count {
                        if let Some(coord) = vacant_iter.next() {
                            if let Some(mut map_data) = world.get_resource_mut::<MapData>() {
                                if let Some(tile) = map_data.tiles.get_mut(&coord) {
                                    if !tile.terrain.allows_forests() {
                                        tile.terrain = TerrainType::Grass;
                                    }
                                    tile.forest_type = ForestType::Deciduous;
                                    tile.forest_density = 0.5;
                                }
                            }
                        }
                    }
                }
                StarterResource::Food => {
                    if let Some(coord) = vacant_iter.next() {
                        if let Some(mut map_data) = world.get_resource_mut::<MapData>() {
                            if let Some(tile) = map_data.tiles.get_mut(&coord) {
                                if tile.terrain != TerrainType::Grass
                                    && tile.terrain != TerrainType::Steppe
                                {
                                    tile.terrain = TerrainType::Grass;
                                }
                            }
                        }

                        let height = if let Some(map_data) = world.get_resource::<MapData>() {
                            map_data.get_hex_height(coord.q, coord.r)
                        } else {
                            0.0
                        };
                        let mut pos = coord.to_world(HEX_SIZE);
                        pos.y = height;

                        let bush_scene = if let Some(assets) = world.get_resource::<GameAssets>() {
                            assets.bush_scene.clone()
                        } else {
                            return;
                        };

                        world.spawn(BerryBushBundle {
                            bush: BerryBush { food_amount: 10.0 },
                            scene: SceneRoot(bush_scene),
                            transform: Transform::from_translation(pos),
                            obstacle: crate::map::navigation::NavObstacle {
                                cost: crate::map::navigation::COST_BLOCKER,
                            },
                        });
                    }
                }
                StarterResource::Flax => {
                    if let Some(coord) = vacant_iter.next() {
                        if let Some(mut map_data) = world.get_resource_mut::<MapData>() {
                            if let Some(tile) = map_data.tiles.get_mut(&coord) {
                                if tile.terrain != TerrainType::Grass
                                    && tile.terrain != TerrainType::Dirt
                                {
                                    tile.terrain = TerrainType::Grass;
                                }
                            }
                        }

                        let height = if let Some(map_data) = world.get_resource::<MapData>() {
                            map_data.get_hex_height(coord.q, coord.r)
                        } else {
                            0.0
                        };
                        let mut pos = coord.to_world(HEX_SIZE);
                        pos.y = height;

                        world.spawn(ResourceDepositBundle {
                            deposit: ResourceDeposit {
                                deposit_type: DepositType::WildFlax,
                                amount: 10,
                                hex_coord: coord,
                                habitat_valid: true,
                            },
                            name: Name::new("WildFlax Deposit"),
                            transform: Transform::from_translation(pos),
                            visibility: Visibility::Visible,
                            inherited_visibility: InheritedVisibility::default(),
                            marker: MapEntity,
                        });
                    }
                }
            }
        }

        if let Some(mut map_data) = world.get_resource_mut::<MapData>() {
            let _ = map_data.bypass_change_detection();
            map_data.set_changed();
        }
        world.write_message(RebuildMeshEvent);
    }
}

pub trait BalanceCommandsExt {
    fn auto_balance_starter_area(&mut self, faction_id: u32) -> &mut Self;
}

impl BalanceCommandsExt for Commands<'_, '_> {
    fn auto_balance_starter_area(&mut self, faction_id: u32) -> &mut Self {
        self.queue(AutoBalanceCommand { faction_id });
        self
    }
}
