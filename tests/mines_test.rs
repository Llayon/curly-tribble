// tests/mines_test.rs
use bevy::prelude::*;
use savage_fantasy::game_state::{EditorPhase, GameState, MineDepth, MineType};
use savage_fantasy::map::data::{OceanState, TerrainType, TileData};
use savage_fantasy::map::mines::{MineBundle, MineDeposit};
use savage_fantasy::map::navigation::NavigationMap;
use savage_fantasy::map::validation_deposits::validate_mines;
use savage_fantasy::map::{HexCoord, LandscapeFeature, MapData, MapEntity};

#[test]
fn test_mines_validation() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(State::new(EditorPhase::Mines));
    app.insert_resource(State::new(GameState::Playing));

    let mut map_data = MapData::default();
    map_data.tiles.insert(
        HexCoord::new(0, 0),
        TileData {
            faction_id: Some(1),
            ocean_state: savage_fantasy::map::data::OceanState::Land,
            terrain: TerrainType::Grass,
            ..default()
        },
    );
    map_data.tiles.insert(
        HexCoord::new(1, 1),
        TileData {
            ocean_state: savage_fantasy::map::data::OceanState::Ocean,
            terrain: TerrainType::Grass,
            ..default()
        },
    );
    map_data.tiles.insert(
        HexCoord::new(2, 2),
        TileData {
            ocean_state: savage_fantasy::map::data::OceanState::Land,
            terrain: TerrainType::Grass,
            landscape_feature: LandscapeFeature::Mountain,
            ..default()
        },
    );

    app.insert_resource(map_data);
    app.insert_resource(NavigationMap::default());
    app.init_resource::<savage_fantasy::map::surface_gameplay::types::SurfaceGameplayMap>();
    app.init_resource::<
        savage_fantasy::map::surface_gameplay::runtime::SurfaceGameplayGenerationState,
    >();

    app.world_mut().spawn(MineBundle {
        deposit: MineDeposit {
            mine_type: MineType::Iron,
            amount: 500,
            depth: MineDepth::Shallow,
            hex_coord: HexCoord::new(2, 2),
        },
        name: Name::new("Valid Iron"),
        transform: Transform::default(),
        global_transform: GlobalTransform::default(),
        visibility: Visibility::Visible,
        inherited_visibility: InheritedVisibility::default(),
        marker: savage_fantasy::map::MapEntity,
    });

    app.world_mut().spawn(MineBundle {
        deposit: MineDeposit {
            mine_type: MineType::Coal,
            amount: 500,
            depth: MineDepth::Shallow,
            hex_coord: HexCoord::new(1, 1),
        },
        name: Name::new("Invalid Ocean Coal"),
        transform: Transform::default(),
        global_transform: GlobalTransform::default(),
        visibility: Visibility::Visible,
        inherited_visibility: InheritedVisibility::default(),
        marker: savage_fantasy::map::MapEntity,
    });

    app.world_mut().spawn(MineBundle {
        deposit: MineDeposit {
            mine_type: MineType::Gold,
            amount: 500,
            depth: MineDepth::Shallow,
            hex_coord: HexCoord::new(0, 0),
        },
        name: Name::new("Invalid Gold Terrain"),
        transform: Transform::default(),
        global_transform: GlobalTransform::default(),
        visibility: Visibility::Visible,
        inherited_visibility: InheritedVisibility::default(),
        marker: savage_fantasy::map::MapEntity,
    });

    let mut schedule = Schedule::new(Update);
    schedule.add_systems(validate_mines);
    app.add_schedule(schedule);
    app.update();

    let map_data = app.world().resource::<MapData>();

    assert!(!map_data.validation_errors.is_empty());

    let has_ocean_error = map_data
        .validation_errors
        .iter()
        .any(|err| err.contains("ocean"));
    let has_terrain_error = map_data
        .validation_errors
        .iter()
        .any(|err| err.contains("Gold vein"));

    assert!(has_ocean_error, "Should flag ocean mine placement");
    assert!(
        has_terrain_error,
        "Should flag invalid gold terrain placement"
    );
}
