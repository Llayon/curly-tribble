use super::algo::*;
use super::types::*;
use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;

pub struct NavigationCommandsPlugin;
impl Plugin for NavigationCommandsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub trait NavigationCommandsExt {
    fn move_to(&mut self, entity: bevy::prelude::Entity, target_pos: Vec3) -> &mut Self;
    fn interact_with(
        &mut self,
        entity: bevy::prelude::Entity,
        target_pos: Vec3,
        radius: f32,
    ) -> &mut Self;
}

impl NavigationCommandsExt for Commands<'_, '_> {
    fn move_to(&mut self, entity: bevy::prelude::Entity, target_pos: Vec3) -> &mut Self {
        self.queue(ComputePathCommand {
            agent: entity,
            target_pos,
            radius: 0.1,
        });
        self
    }

    fn interact_with(
        &mut self,
        entity: bevy::prelude::Entity,
        target_pos: Vec3,
        radius: f32,
    ) -> &mut Self {
        self.queue(ComputePathCommand {
            agent: entity,
            target_pos,
            radius,
        });
        self
    }
}

pub struct ComputePathCommand {
    pub agent: bevy::prelude::Entity,
    pub target_pos: Vec3,
    pub radius: f32,
}

impl Command for ComputePathCommand {
    fn apply(self, world: &mut World) {
        let start_pos = if let Some(t) = world.get::<Transform>(self.agent) {
            t.translation
        } else {
            return;
        };

        let nav_map_res = if let Some(map) = world.get_resource::<NavigationMap>() {
            map
        } else {
            return;
        };

        let grid = nav_map_res.grid.clone();
        let thread_pool = AsyncComputeTaskPool::get();
        let target_pos = self.target_pos;
        let radius = self.radius;

        let task = thread_pool
            .spawn(async move { compute_astar_path(&grid, start_pos, target_pos, radius) });

        world
            .commands()
            .entity(self.agent)
            .insert(ComputingPath(task));
    }
}
