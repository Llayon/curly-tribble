use bevy::prelude::*;
use bevy::tasks::Task;
use std::collections::HashMap;

pub struct NavigationTypesPlugin;
impl Plugin for NavigationSystemsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub const COST_BLOCKER: u8 = 0;
pub const _COST_ROAD: u8 = 10;
pub const COST_BASE: u8 = 20;
pub const AGENT_HEIGHT: f32 = 0.4;

#[derive(Resource, Default, Debug)]
pub struct NavigationMap {
    pub grid: HashMap<IVec2, u8>,
}

#[derive(Component, Debug, Clone, Copy, Default)]
#[require(Transform)]
pub struct NavObstacle {
    pub cost: u8,
}

#[derive(Component, Debug, Default)]
pub struct Path {
    pub points: Vec<Vec3>,
    pub current_index: usize,
}

#[derive(Component)]
pub struct ComputingPath(pub Task<Option<Vec<Vec3>>>);

#[derive(Message)]
pub struct PathBlockEvent {
    pub cell: IVec2,
}

#[must_use]
pub fn world_to_grid(pos: Vec3) -> IVec2 {
    IVec2::new(pos.x.round() as i32, pos.z.round() as i32)
}

#[must_use]
pub fn grid_to_world(cell: IVec2, map: &crate::map::MapData) -> Vec3 {
    let elevation = map
        .get_tile(cell.x, cell.y)
        .map_or(0.0, |t| t.elevation);

    Vec3::new(
        cell.x as f32,
        (elevation * crate::map::MAX_HEIGHT) + AGENT_HEIGHT,
        cell.y as f32,
    )
}
