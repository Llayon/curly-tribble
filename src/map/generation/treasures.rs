use crate::map::data::OceanState;
use crate::map::{
    HexCoord, HiddenTreasure, MapData, MapEntity, ResourceType, TreasureBundle, TreasureDeposit,
    TreasureItem, VisibleTreasure, HEX_SIZE,
};
use bevy::prelude::*;
use rand::prelude::*;

pub struct TreasuresGenerationPlugin;

impl Plugin for TreasuresGenerationPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn auto_spawn_treasures(commands: &mut Commands, map_data: &MapData, seed: u32) {
    let mut rng = rand::rngs::StdRng::seed_from_u64(u64::from(seed + 300));

    let land_tiles: Vec<HexCoord> = map_data
        .tiles
        .iter()
        .filter(|(_, t)| t.ocean_state == OceanState::Land)
        .map(|(c, _)| *c)
        .collect();

    if land_tiles.is_empty() {
        return;
    }

    // 1. Visible treasures (Ruins) near center
    let mut sum_q = 0;
    let mut sum_r = 0;
    for c in &land_tiles {
        sum_q += c.q;
        sum_r += c.r;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let center = HexCoord::new(
        sum_q / land_tiles.len() as i32,
        sum_r / land_tiles.len() as i32,
    );

    let mut sorted_land = land_tiles.clone();
    sorted_land.sort_by_key(|c| c.distance(center));

    let num_visible = rng.gen_range(1..=2);
    let mut occupied = std::collections::HashSet::new();

    for i in 0..num_visible {
        if let Some(&coord) = sorted_land.get(i) {
            let contents = if rng.gen_bool(0.7) {
                vec![
                    TreasureItem::Gold(rng.gen_range(50..150)),
                    crate::map::treasures::TreasureItem::ArtifactDef(
                        crate::map::treasures::ArtifactType::AncientRelic,
                    ),
                ]
            } else {
                vec![TreasureItem::Resources(
                    ResourceType::Wood,
                    rng.gen_range(30..80),
                )]
            };
            spawn_treasure(commands, coord, TreasureVisibility::Visible, contents);
            occupied.insert(coord);
        }
    }

    // 2. Hidden treasures scattered
    let num_hidden = rng.gen_range(2..=4);
    let mut hidden_count = 0;
    let mut attempts = 0;
    while hidden_count < num_hidden && attempts < 20 {
        attempts += 1;
        if let Some(&coord) = land_tiles.choose(&mut rng) {
            if occupied.contains(&coord) {
                continue;
            }
            let contents = if rng.gen_bool(0.5) {
                vec![TreasureItem::Gold(rng.gen_range(20..60))]
            } else {
                vec![TreasureItem::Resources(
                    ResourceType::Wood,
                    rng.gen_range(15..40),
                )]
            };
            spawn_treasure(commands, coord, TreasureVisibility::Hidden, contents);
            occupied.insert(coord);
            hidden_count += 1;
        }
    }
}

#[derive(PartialEq)]
enum TreasureVisibility {
    Visible,
    Hidden,
}

fn spawn_treasure(
    commands: &mut Commands,
    coord: HexCoord,
    visibility: TreasureVisibility,
    contents: Vec<TreasureItem>,
) {
    let mut entity = commands.spawn(TreasureBundle {
        deposit: TreasureDeposit {
            contents,
            hex_coord: coord,
        },
        name: Name::new(if visibility == TreasureVisibility::Hidden {
            "Hidden Treasure"
        } else {
            "Visible Treasure (Ruins)"
        }),
        map_entity: MapEntity,
        transform: Transform::from_translation(coord.to_world(HEX_SIZE)),
        global_transform: GlobalTransform::default(),
        visibility: Visibility::Visible,
        inherited_visibility: InheritedVisibility::default(),
    });

    if visibility == TreasureVisibility::Hidden {
        entity.insert(HiddenTreasure);
    } else {
        entity.insert(VisibleTreasure);
    }
}
