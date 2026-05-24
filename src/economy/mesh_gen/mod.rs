// src/economy/mesh_gen/mod.rs
pub mod generator;
pub mod gizmos;

use crate::game_state::EditorPhase;
use crate::map::zoning::{GlobalTerrainBundle, Roof, WaterBundle};
use crate::map::{MapData, MapEntity};
use bevy::prelude::*;
use generator::create_global_map_meshes;

pub struct MeshGenPlugin;

#[derive(Resource, Default)]
pub struct GeneratedMapAssets {
    pub terrain: Option<Handle<Mesh>>,
    pub water: Option<Handle<Mesh>>,
    pub roof: Option<Handle<Mesh>>,
}

impl Plugin for MeshGenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GeneratedMapAssets>().add_systems(
            Update,
            (
                gizmos::draw_hex_grid_gizmos.run_if(not(in_state(EditorPhase::Height3D))),
                gizmos::draw_factions_gizmos.run_if(in_state(EditorPhase::Factions)),
                gizmos::draw_cliffs_gizmos.run_if(|phase: Res<State<EditorPhase>>| {
                    *phase.get() >= EditorPhase::Landscape
                }),
                gizmos::draw_forest_gizmos.run_if(|phase: Res<State<EditorPhase>>| {
                    *phase.get() >= EditorPhase::Sediments
                }),
                gizmos::draw_npc_objects_gizmos
                    .run_if(|phase: Res<State<EditorPhase>>| *phase.get() >= EditorPhase::NPCs),
            ),
        );
    }
}

pub struct SpawnGlobalTerrainCommand {
    pub map_data: MapData,
    pub phase: EditorPhase,
    pub faction_manager: crate::game_state::FactionManager,
    pub config: crate::map::terrain_gen::TerrainConfig,
}

impl Command for SpawnGlobalTerrainCommand {
    fn apply(self, world: &mut World) {
        let is_flat = self.phase != EditorPhase::Height3D;

        let old_handles =
            if let Some(mut gen_assets) = world.get_resource_mut::<GeneratedMapAssets>() {
                (
                    gen_assets.terrain.take(),
                    gen_assets.water.take(),
                    gen_assets.roof.take(),
                )
            } else {
                (None, None, None)
            };

        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        if let Some(h) = old_handles.0 {
            meshes.remove(&h);
        }
        if let Some(h) = old_handles.1 {
            meshes.remove(&h);
        }
        if let Some(h) = old_handles.2 {
            meshes.remove(&h);
        }

        let (mesh, water_mesh, roof_mesh) = create_global_map_meshes(
            &self.map_data,
            self.phase,
            &self.faction_manager,
            &self.config,
        );

        let terrain_handle = meshes.add(mesh);
        let water_handle = meshes.add(water_mesh);
        let roof_handle = meshes.add(roof_mesh);

        if let Some(mut gen_assets) = world.get_resource_mut::<GeneratedMapAssets>() {
            gen_assets.terrain = Some(terrain_handle.clone());
            gen_assets.water = Some(water_handle.clone());
            gen_assets.roof = Some(roof_handle.clone());
        }

        let assets = world.resource::<crate::economy::GameAssets>();
        let ground_mat = assets.ground_material.clone();
        let water_mat = assets.water_material.clone();
        let mountain_mat = assets.mountain_material.clone();

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        if let Some(mat) = materials.get_mut(&ground_mat) {
            mat.unlit = is_flat;
        }
        if let Some(mat) = materials.get_mut(&water_mat) {
            mat.unlit = is_flat;
        }

        world.spawn(GlobalTerrainBundle {
            mesh: Mesh3d(terrain_handle),
            material: MeshMaterial3d(ground_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            name: Name::new("Global Terrain"),
            marker: MapEntity,
        });

        world.spawn(WaterBundle {
            mesh: Mesh3d(water_handle),
            material: MeshMaterial3d(water_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            name: Name::new("Water Layer"),
            marker: MapEntity,
        });

        world.spawn(crate::map::zoning::MountainRoofBundle {
            mesh: Mesh3d(roof_handle),
            material: MeshMaterial3d(mountain_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::default(),
            inherited_visibility: InheritedVisibility::default(),
            roof: Roof,
            name: Name::new("Global Mountain Roofs"),
            marker: MapEntity,
        });
    }
}
