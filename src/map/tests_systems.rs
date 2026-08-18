// src/map/tests_systems.rs
//! Unit tests for `src/map/systems.rs` map lifecycle systems.

#[cfg(test)]
pub mod tests {
    use crate::economy::GameAssets;
    use crate::game_state::{EditorPhase, FactionManager};
    use crate::map::face_topology::types::HexFaceTopology;
    use crate::map::surface_gameplay::runtime::{
        SurfaceGameplayGenerationOutcome, SurfaceGameplayGenerationState,
    };
    use crate::map::systems::handle_rebuild_mesh;
    use crate::map::terrain_bake::runtime::{
        TerrainBakeGenerationOutcome, TerrainBakeGenerationState,
    };
    use crate::map::terrain_bake::types::SurfaceTerrainBake;
    use crate::map::terrain_gen::TerrainConfig;
    use crate::map::{MapData, MapEntity, MapVisualEntity, RebuildMeshEvent};
    use bevy::prelude::*;

    #[allow(dead_code)]
    pub struct MapSystemsTestsPlugin;

    impl Plugin for MapSystemsTestsPlugin {
        fn build(&self, _app: &mut App) {}
    }

    fn request_rebuild(mut events: MessageWriter<RebuildMeshEvent>) {
        events.write(RebuildMeshEvent);
    }

    #[test]
    fn rebuild_replaces_only_visual_map_entities() {
        let mut app = App::new();
        app.insert_resource(MapData::default())
            .insert_resource(FactionManager::default())
            .insert_resource(TerrainConfig::default())
            .init_resource::<HexFaceTopology>()
            .init_resource::<SurfaceTerrainBake>()
            .init_resource::<TerrainBakeGenerationState>()
            .init_resource::<SurfaceGameplayGenerationState>()
            .insert_resource(State::new(EditorPhase::Shape))
            .insert_resource(GameAssets::default())
            .insert_resource(Assets::<Mesh>::default())
            .insert_resource(Assets::<StandardMaterial>::default())
            .add_message::<RebuildMeshEvent>()
            .add_systems(Update, (request_rebuild, handle_rebuild_mesh).chain());

        app.world_mut()
            .resource_mut::<TerrainBakeGenerationState>()
            .last_outcome = TerrainBakeGenerationOutcome::Success;
        app.world_mut()
            .resource_mut::<SurfaceGameplayGenerationState>()
            .last_outcome = SurfaceGameplayGenerationOutcome::Success;

        let visual_entity = app.world_mut().spawn((MapEntity, MapVisualEntity)).id();
        let content_entity = app.world_mut().spawn(MapEntity).id();

        app.update();

        assert!(app.world().get_entity(visual_entity).is_err());
        assert!(app.world().get_entity(content_entity).is_ok());
    }
}
