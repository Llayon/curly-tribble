use crate::map::zoning::{SmoothTileBundle, Tile};
use bevy::prelude::*;

pub struct MeshGenPlugin;

impl Plugin for MeshGenPlugin {
    fn build(&self, _app: &mut App) {}
}

pub struct SpawnSmoothTileCommand {
    pub x: i32,
    pub z: i32,
    pub h_nw: f32,
    pub h_ne: f32,
    pub h_sw: f32,
    pub h_se: f32,
    pub material: Handle<StandardMaterial>,
    pub terrain: crate::map::zoning::TerrainType,
}

impl Command for SpawnSmoothTileCommand {
    fn apply(self, world: &mut World) {
        let mesh = create_smooth_tile_mesh(self.h_nw, self.h_ne, self.h_sw, self.h_se);
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        let mesh_handle = meshes.add(mesh);

        world.spawn(SmoothTileBundle {
            mesh: Mesh3d(mesh_handle),
            material: MeshMaterial3d(self.material),
            transform: Transform::from_xyz(self.x as f32, 0.0, self.z as f32),
            tile: Tile {
                terrain: self.terrain,
            },
        });
    }
}

/// Генерирует меш для одного тайла со сглаженными углами.
pub fn create_smooth_tile_mesh(h_nw: f32, h_ne: f32, h_sw: f32, h_se: f32) -> Mesh {
    let mut mesh = Mesh::from(Plane3d::default());

    let vertices = vec![
        [-0.5, h_nw, -0.5], // 0: NW
        [0.5, h_ne, -0.5],  // 1: NE
        [-0.5, h_sw, 0.5],  // 2: SW
        [0.5, h_se, 0.5],   // 3: SE
    ];

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.duplicate_vertices();
    mesh.compute_flat_normals();

    mesh
}
