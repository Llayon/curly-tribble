// src/map/mod.rs
pub mod atmosphere;
pub mod camps;
pub mod construction;
pub mod data;
pub mod deposits;
pub mod factions;
pub mod hex_math;
pub mod navigation;
pub mod poi;
pub mod resources;
pub mod river_gen;
pub mod terrain_gen;
pub mod visibility;
pub mod zoning;

pub mod generation;
pub mod systems;
pub mod tools;
pub mod validation;

use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;
pub use camps::{EnemyCamp, EnemyCampBundle};
pub use data::{
    EdgeCoord, EdgeData, ForestType, LandscapeFeature, MapData, SedimentTraits, TerrainType,
    TileData, WorldSeed, HEX_SIZE, MAX_HEIGHT,
};
pub use deposits::{DepositType, ResourceDeposit, ResourceDepositBundle};
pub use factions::{FactionMarker, FactionMarkerBundle};
pub use hex_math::HexCoord;
pub use poi::{PoiBundle, PoiType, PointOfInterest};
use terrain_gen::{TerrainConfig, TerrainGenerator};
pub use zoning::Tile;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct MapEntity;

#[derive(Message)]
pub struct GenerateMapEvent {
    pub force_reset: bool,
    pub auto_fill_phase: Option<crate::game_state::EditorPhase>,
}

#[derive(Message)]
pub struct RebuildMeshEvent;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let config = TerrainConfig::default();
        app.insert_resource(TerrainGenerator::new(config.seed))
            .insert_resource(config)
            .register_type::<TerrainConfig>()
            .register_type::<MapEntity>()
            .add_plugins(bevy_inspector_egui::quick::ResourceInspectorPlugin::<
                TerrainConfig,
            >::default())
            .add_message::<GenerateMapEvent>()
            .add_message::<RebuildMeshEvent>()
            .add_plugins((
                zoning::ZoningPlugin,
                resources::ResourcesPlugin,
                construction::ConstructionPlugin,
                navigation::NavigationPlugin,
                visibility::VisibilityPlugin,
                crate::economy::mesh_gen::MeshGenPlugin,
                river_gen::RiverGenPlugin,
                terrain_gen::TerrainGenPlugin,
            ))
            .add_systems(
                Startup,
                (|mut ev: MessageWriter<GenerateMapEvent>| {
                    ev.write(GenerateMapEvent {
                        force_reset: true,
                        auto_fill_phase: None,
                    });
                })
                .in_set(StartupSet::SpawnEntities),
            )
            .add_systems(
                Update,
                (
                    systems::handle_regeneration.in_set(GameSet::Logic),
                    systems::handle_rebuild_mesh.in_set(GameSet::Logic),
                    tools::handle_shape_tools.in_set(GameSet::Logic),
                    tools::handle_faction_tools.in_set(GameSet::Logic),
                    tools::handle_landscape_tools.in_set(GameSet::Logic),
                    tools::handle_sediment_tools.in_set(GameSet::Logic),
                    tools::handle_bio_tools.in_set(GameSet::Logic),
                    tools::handle_npc_tools.in_set(GameSet::Logic),
                    systems::handle_faction_auto_relocation.in_set(GameSet::Logic),
                    validation::validate_faction_placements.in_set(GameSet::Logic),
                    validation::validate_bio_habitats.in_set(GameSet::Logic),
                    systems::monitor_inspector_triggers
                        .run_if(resource_changed::<TerrainConfig>)
                        .in_set(GameSet::Logic),
                    validation::rebuild_map_on_phase_change
                        .run_if(state_changed::<crate::game_state::EditorPhase>)
                        .in_set(GameSet::Logic),
                ),
            );
    }
}
