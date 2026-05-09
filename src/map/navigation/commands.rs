use super::algo::compute_astar_path;
use super::types::{ComputingPath, NavigationMap};
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;

pub struct NavigationCommandsPlugin;
impl Plugin for NavigationCommandsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub trait NavigationCommandsExt {
    fn move_to(&mut self, entity: Entity, target_pos: Vec3) -> &mut Self;
    fn interact_with(&mut self, entity: Entity, target_pos: Vec3, radius: f32) -> &mut Self;
}

impl NavigationCommandsExt for Commands<'_, '_> {
    fn move_to(&mut self, entity: Entity, target_pos: Vec3) -> &mut Self {
        self.queue(ComputePathCommand {
            agent: entity,
            target_pos,
            radius: 0.1,
        });
        self
    }

    fn interact_with(&mut self, entity: Entity, target_pos: Vec3, radius: f32) -> &mut Self {
        self.queue(ComputePathCommand {
            agent: entity,
            target_pos,
            radius,
        });
        self
    }
}

type NavEntity = Entity;

pub struct ComputePathCommand {
    pub agent: NavEntity,
    pub target_pos: Vec3,
    pub radius: f32,
}

impl Command for ComputePathCommand {
    fn apply(self, world: &mut World) {
        let Some(t) = world.get::<Transform>(self.agent) else {
            return;
        };
        let start_pos = t.translation;

        let Some(nav_map_res) = world.get_resource::<NavigationMap>() else {
            return;
        };

        let Some(map_data) = world.get_resource::<crate::map::MapData>() else {
            return;
        };
        let map_data = map_data.clone();

        let grid = nav_map_res.grid.clone();
        let thread_pool = AsyncComputeTaskPool::get();
        let target_pos = self.target_pos;
        let radius = self.radius;

        let task = thread_pool.spawn(async move {
            compute_astar_path(&grid, start_pos, target_pos, radius, &map_data)
        });

        // ВАЖНО: Вставляем компонент немедленно через World, а не через очередь команд,
        // чтобы избежать Race Condition в FixedUpdate.
        if let Ok(mut entity) = world.get_entity_mut(self.agent) {
            entity.insert(ComputingPath(task));
        }
    }
}
