// tests/map_export_test.rs
use bevy::prelude::*;
use savage_fantasy::game_state::{EditorPhase, GameState};
use savage_fantasy::map::export::{export_map_to_json, import_map_from_json, MapExportPlugin};
use savage_fantasy::map::{HexCoord, MapData, RebuildMeshEvent, TileData};

#[test]
fn test_map_export_and_import() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(State::new(EditorPhase::Export));
    app.insert_resource(State::new(GameState::Playing));

    app.add_plugins(MapExportPlugin);
    app.add_message::<RebuildMeshEvent>();

    let mut map_data = MapData::default();
    map_data.width = 12;
    map_data.height = 12;

    map_data.tiles.insert(
        HexCoord::new(1, 2),
        TileData {
            elevation: 1.5,
            faction_id: Some(1),
            ..default()
        },
    );

    let test_dir = std::env::temp_dir().join("savage_fantasy_test_export");
    let export_path = test_dir.join("test_map.json");

    // 1. Test export
    let export_res = export_map_to_json(&map_data, &export_path);
    assert!(export_res.is_ok(), "Map export to JSON must succeed");
    assert!(export_path.exists(), "Exported JSON file must exist");

    // 2. Test import
    let import_res = import_map_from_json(&export_path);
    assert!(import_res.is_ok(), "Map import from JSON must succeed");

    let package = import_res.unwrap();
    assert_eq!(package.width, 12);
    assert_eq!(package.height, 12);
    assert!(!package.tiles.is_empty());

    let tile = package.tiles.iter().find(|t| t.q == 1 && t.r == 2);
    assert!(tile.is_some());
    assert_eq!(tile.unwrap().faction_id, Some(1));

    // Cleanup
    let _ = std::fs::remove_dir_all(test_dir);
}
