// src/economy/mesh_gen/mod.rs
pub mod bake;
pub mod billboards;
pub mod cliff_gizmos;
pub mod generator;
pub mod gizmos;
pub mod overlay;
pub mod treasures;

#[cfg(test)]
mod tests_cliff_gizmos;
#[cfg(test)]
mod tests_overlay;

use crate::game_state::EditorPhase;
use crate::map::zoning::{GlobalTerrainBundle, Roof, WaterBundle};
use crate::map::{MapData, MapEntity, MapVisualEntity};
use bevy::prelude::*;
use generator::create_global_map_meshes_from_bake;

pub struct MeshGenPlugin;

#[derive(Resource, Default)]
pub struct GeneratedMapAssets {
    pub terrain: Option<Handle<Mesh>>,
    pub water: Option<Handle<Mesh>>,
    pub roof: Option<Handle<Mesh>>,
}

impl Plugin for MeshGenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GeneratedMapAssets>()
            .add_plugins((
                billboards::BillboardPlugin,
                cliff_gizmos::CliffGizmosPlugin,
                generator::MeshGeneratorPlugin,
                gizmos::GizmosPlugin,
                treasures::TreasureMeshPlugin,
            ))
            .add_systems(
                Update,
                (
                    gizmos::draw_hex_grid_gizmos,
                    gizmos::draw_factions_gizmos.run_if(in_state(EditorPhase::Factions)),
                    cliff_gizmos::draw_cliffs_gizmos
                        .after(crate::map::face_topology::runtime::rebuild_bound_cliff_edges)
                        .run_if(|phase: Res<State<EditorPhase>>| {
                            *phase.get() >= EditorPhase::Landscape
                        }),
                    cliff_gizmos::draw_hovered_cliff_gizmo.run_if(in_state(EditorPhase::Landscape)),
                    gizmos::draw_forest_gizmos.run_if(|phase: Res<State<EditorPhase>>| {
                        *phase.get() >= EditorPhase::Sediments
                    }),
                    gizmos::draw_npc_objects_gizmos
                        .run_if(|phase: Res<State<EditorPhase>>| *phase.get() >= EditorPhase::NPCs),
                    gizmos::draw_mines_gizmos.run_if(|phase: Res<State<EditorPhase>>| {
                        *phase.get() >= EditorPhase::Mines
                    }),
                ),
            );
    }
}

pub struct SpawnGlobalTerrainCommand {
    pub topology: crate::map::topology::TerrainTopology,
    pub face_topology: crate::map::face_topology::types::HexFaceTopology,
    pub map_data: MapData,
    pub phase: EditorPhase,
    pub faction_manager: crate::game_state::FactionManager,
    pub config: crate::map::terrain_gen::TerrainConfig,
    /// M5.1 authoritative ground geometry. Rebuild is fail-closed: the command
    /// is only constructed from a validated, successfully generated bake.
    pub bake: crate::map::terrain_bake::types::SurfaceTerrainBake,
}

impl Command for SpawnGlobalTerrainCommand {
    #[allow(clippy::too_many_lines)]
    fn apply(self, world: &mut World) {
        // 1. GENERATE FIRST (pure): on failure nothing is touched, so a failed
        //    bake rebuild leaves the previously rendered terrain fully intact —
        //    including the previously published TerrainTopology resource.
        let generated = create_global_map_meshes_from_bake(
            &self.map_data,
            &self.bake,
            &self.face_topology,
            self.phase,
            &self.faction_manager,
            &self.config,
        );
        let (mesh, water_mesh, roof_mesh) = match generated {
            Ok(m) => m,
            Err(err) => {
                bevy::log::tracing::event!(
                    bevy::log::tracing::Level::ERROR,
                    error = ?err,
                    "Failed to create map meshes from SurfaceTerrainBake; old terrain remains"
                );
                return;
            }
        };

        // 2. SWAP: publish the new topology and retire old assets/visuals only
        //    after new geometry exists.
        world.insert_resource(self.topology.clone());
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

        {
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
        }

        let mut old_visuals = world.query_filtered::<Entity, With<MapVisualEntity>>();
        let old_visual_ids: Vec<Entity> = old_visuals.iter(world).collect();
        for entity in old_visual_ids {
            world.entity_mut(entity).despawn();
        }

        let min_elev = self
            .map_data
            .tiles
            .values()
            .map(|t| t.elevation)
            .fold(f32::INFINITY, f32::min);
        let max_elev = self
            .map_data
            .tiles
            .values()
            .map(|t| t.elevation)
            .fold(f32::NEG_INFINITY, f32::max);

        let (min_mesh_y, max_mesh_y) =
            if let Some(bevy::mesh::VertexAttributeValues::Float32x3(pos)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            {
                let min_y = pos.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
                let max_y = pos.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
                (min_y, max_y)
            } else {
                (0.0, 0.0)
            };

        let (exact_up_normals, sloped_normals) =
            if let Some(bevy::mesh::VertexAttributeValues::Float32x3(nor)) =
                mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
            {
                let exact_up = nor
                    .iter()
                    .filter(|n| n[0].abs() < 1e-4 && (n[1] - 1.0).abs() < 1e-4 && n[2].abs() < 1e-4)
                    .count();
                let sloped = nor
                    .iter()
                    .filter(|n| n[0].abs() > 1e-4 || n[2].abs() > 1e-4)
                    .count();
                (exact_up, sloped)
            } else {
                (0, 0)
            };

        let is_flat = self.phase < EditorPhase::Height3D;

        let res_tri_count = world
            .get_resource::<crate::map::topology::TerrainTopology>()
            .map_or(0, |t| t.triangles.len());

        debug!(
            "TERRAIN REBUILD DIAGNOSTICS [Phase: {:?}]: TileCount={}, TopVerts={}, TopTris={}, ResTris={}, MinElev={:.3}, MaxElev={:.3}, MinMeshY={:.3}, MaxMeshY={:.3}, GroundUnlit={}, ExactUpNormals={}, SlopedNormals={}, BakeVerts={}, BakeFaces={}, BakeWalls={}",
            self.phase,
            self.map_data.tiles.len(),
            self.topology.vertices_xz.len(),
            self.topology.triangles.len(),
            res_tri_count,
            min_elev,
            max_elev,
            min_mesh_y,
            max_mesh_y,
            is_flat,
            exact_up_normals,
            sloped_normals,
            self.bake.vertices.len(),
            self.bake.faces.len(),
            self.bake.cliff_walls.len(),
        );

        let (terrain_handle, water_handle, roof_handle) = {
            let mut meshes = world.resource_mut::<Assets<Mesh>>();
            let t_h = meshes.add(mesh);
            let w_h = water_mesh.map(|m| meshes.add(m));
            let r_h = roof_mesh.map(|m| meshes.add(m));
            (t_h, w_h, r_h)
        };

        if let Some(mut gen_assets) = world.get_resource_mut::<GeneratedMapAssets>() {
            gen_assets.terrain = Some(terrain_handle.clone());
            gen_assets.water.clone_from(&water_handle);
            gen_assets.roof.clone_from(&roof_handle);
        }

        let assets = world.resource::<crate::economy::GameAssets>();
        let ground_mat = assets.ground_material.clone();
        let water_mat = assets.water_material.clone();
        let mountain_mat = assets.mountain_material.clone();

        let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
        if let Some(mat) = materials.get_mut(&ground_mat) {
            mat.unlit = is_flat;
            mat.double_sided = true;
            mat.cull_mode = None;
        }
        if let Some(mat) = materials.get_mut(&water_mat) {
            mat.unlit = true;
            mat.double_sided = true;
            mat.cull_mode = None;
        }

        world.spawn(GlobalTerrainBundle {
            mesh: Mesh3d(terrain_handle),
            material: MeshMaterial3d(ground_mat),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            visibility: Visibility::Visible,
            inherited_visibility: InheritedVisibility::default(),
            name: Name::new("Global Terrain"),
            marker: MapEntity,
            visual_marker: MapVisualEntity,
        });

        if let Some(water_handle) = water_handle {
            world.spawn(WaterBundle {
                mesh: Mesh3d(water_handle),
                material: MeshMaterial3d(water_mat),
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                visibility: Visibility::Visible,
                inherited_visibility: InheritedVisibility::default(),
                name: Name::new("Water Layer"),
                marker: MapEntity,
                visual_marker: MapVisualEntity,
            });
        }

        if let Some(roof_handle) = roof_handle {
            world.spawn(crate::map::zoning::MountainRoofBundle {
                mesh: Mesh3d(roof_handle),
                material: MeshMaterial3d(mountain_mat),
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                visibility: Visibility::default(),
                inherited_visibility: InheritedVisibility::default(),
                roof: Roof,
                name: Name::new("Global Mountain Roofs"),
                marker: MapEntity,
                visual_marker: MapVisualEntity,
            });
        }
    }
}
