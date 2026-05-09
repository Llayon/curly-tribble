use super::types::*;
use bevy::prelude::*;
use futures_lite::future;

pub struct NavigationSystemsPlugin;
impl Plugin for NavigationSystemsPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn poll_path_tasks(
    mut commands: Commands,
    mut query: Query<(bevy::prelude::Entity, &mut ComputingPath), With<ComputingPath>>,
) {
    for (entity, mut task) in &mut query {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).remove::<ComputingPath>();
            if let Some(points) = result {
                commands.entity(entity).insert(Path {
                    points,
                    current_index: 0,
                });
            }
        }
    }
}

pub fn follow_path(
    mut commands: Commands,
    mut query: Query<(bevy::prelude::Entity, &mut Transform, &mut Path), With<Path>>,
    time: Res<Time>,
    map: Res<crate::map::MapData>,
) {
    for (entity, mut transform, mut path) in &mut query {
        if path.current_index >= path.points.len() {
            commands.entity(entity).remove::<Path>();
            continue;
        }

        let target = path.points[path.current_index];
        let diff = target - transform.translation;
        let distance = diff.length();
        let speed = 3.0;

        if distance < 0.1 {
            path.current_index += 1;
        } else {
            let move_dir = diff.normalize();
            transform.translation += move_dir * speed * time.delta_secs();

            let grid_pos = world_to_grid(transform.translation);
            if let Some(tile) = map.get_tile(grid_pos.x, grid_pos.y) {
                let target_y = (tile.elevation * crate::map::MAX_HEIGHT) + AGENT_HEIGHT;
                transform.translation.y = target_y;
            }

            transform.look_to(move_dir, Vec3::Y);
        }
    }
}

pub fn handle_path_invalidation(
    mut commands: Commands,
    mut events: MessageReader<PathBlockEvent>,
    paths: Query<(bevy::prelude::Entity, &Path), With<Path>>,
) {
    for event in events.read() {
        for (entity, path) in &paths {
            let is_blocked = path.points[path.current_index..]
                .iter()
                .any(|&p| world_to_grid(p) == event.cell);

            if is_blocked {
                commands.entity(entity).remove::<Path>();
            }
        }
    }
}
