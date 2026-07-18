// src/map/mod.rs
pub mod artifacts;
pub mod atmosphere;
pub mod camps;
pub mod construction;
pub mod data;
pub mod deposits;
pub mod factions;
pub mod hex_math;
pub mod navigation;
pub mod phase_transitions;
pub mod poi;
pub mod resources;
pub mod river_gen;
pub mod terrain_gen;
pub mod treasures;
pub mod visibility;
pub mod zoning;

pub mod generation;
pub mod systems;
pub mod tools;
pub mod validation;
pub mod validation_deposits;

use crate::sets::{GameSet, StartupSet};
pub use artifacts::{
    Artifact, ArtifactBundle, ArtifactLocation, StoredInTreasure, StoresArtifacts, TradeConfig,
};
use bevy::prelude::*;
pub use camps::{EnemyCamp, EnemyCampBundle};
pub use data::{
    EdgeCoord, EdgeData, EdgeDirection, EdgeType, ForestType, LandscapeFeature, MapData,
    TerrainType, TileData, WorldSeed, HEX_SIZE, MAX_HEIGHT,
};
pub use deposits::{DepositType, ResourceDeposit, ResourceDepositBundle};
pub use factions::{FactionMarker, FactionMarkerBundle};
pub use hex_math::HexCoord;
pub use poi::{PoiBundle, PoiType, PointOfInterest};
use terrain_gen::{TerrainConfig, TerrainGenerator};
pub use treasures::{
    ArtifactType, ContainedByTreasure, ContainsArtifact, HiddenTreasure, LinkToolState,
    MapToTarget, ResourceType, TargetEntity, TargetedByTreasureMap, Targeting, TreasureBundle,
    TreasureDeposit, TreasureItem, VisibleTreasure,
};
pub use zoning::Tile;

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct MapEntity;

/// Marker for mesh entities that can be replaced after terrain edits.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct MapVisualEntity;

#[derive(Message)]
pub struct GenerateMapEvent {
    pub mode: GenerationMode,
    pub auto_fill_phase: Option<crate::game_state::EditorPhase>,
}

/// Controls whether generation starts from a fresh world or preserves editor data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationMode {
    Reset,
    Preserve,
}

#[derive(Message)]
pub struct RebuildMeshEvent;

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        let config = TerrainConfig::default();
        app.insert_resource(TerrainGenerator::new(config.seed))
            .insert_resource(config)
            .init_resource::<MapData>()
            .init_resource::<WorldSeed>()
            .init_resource::<navigation::NavigationMap>()
            .init_resource::<LinkToolState>()
            .register_type::<TerrainConfig>()
            .register_type::<MapEntity>()
            .register_type::<MapVisualEntity>()
            .register_type::<treasures::ResourceType>()
            .register_type::<treasures::ArtifactType>()
            .register_type::<treasures::TreasureItem>()
            .register_type::<treasures::TreasureDeposit>()
            .register_type::<treasures::VisibleTreasure>()
            .register_type::<treasures::HiddenTreasure>()
            .register_type::<treasures::Targeting>()
            .register_type::<treasures::TargetedByTreasureMap>()
            .register_type::<treasures::MapToTarget>()
            .register_type::<treasures::ContainsArtifact>()
            .register_type::<treasures::ContainedByTreasure>()
            .register_type::<artifacts::Artifact>()
            .register_type::<artifacts::ArtifactLocation>()
            .register_type::<artifacts::StoredInTreasure>()
            .register_type::<artifacts::StoresArtifacts>()
            .register_type::<artifacts::TradeConfig>()
            .register_type::<LinkToolState>()
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
                        mode: GenerationMode::Reset,
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
                    tools::handle_treasure_tools.in_set(GameSet::Logic),
                    tools::handle_artifact_tools.in_set(GameSet::Logic),
                    systems::handle_faction_auto_relocation.in_set(GameSet::Logic),
                    validation::validate_faction_placements.in_set(GameSet::Logic),
                    validation_deposits::validate_bio_habitats.in_set(GameSet::Logic),
                    validation_deposits::validate_treasures.in_set(GameSet::Logic),
                    systems::monitor_inspector_triggers
                        .run_if(resource_changed::<TerrainConfig>)
                        .in_set(GameSet::Logic),
                    phase_transitions::rebuild_map_on_phase_change
                        .run_if(state_changed::<crate::game_state::EditorPhase>)
                        .in_set(GameSet::Logic),
                ),
            );
    }
}
