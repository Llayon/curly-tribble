use bevy::prelude::*;

pub mod algo;
pub mod commands;
pub mod systems;
pub mod types;

pub use algo::*;
pub use commands::*;
pub use systems::*;
pub use types::*;

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavigationMap>()
            .add_message::<PathBlockEvent>()
            .add_plugins((
                NavigationTypesPlugin,
                NavigationAlgoPlugin,
                NavigationCommandsPlugin,
                NavigationSystemsPlugin,
            ))
            .add_systems(
                Update,
                (poll_path_tasks, follow_path, handle_path_invalidation),
            );

        // Реактивное обновление карты при спавне препятствий
        app.world_mut().spawn(Observer::new(
            |trigger: On<Add, NavObstacle>,
             mut nav: ResMut<NavigationMap>,
             query: Query<(&Transform, &NavObstacle)>,
             mut events: MessageWriter<PathBlockEvent>| {
                if let Ok((transform, obstacle)) = query.get(trigger.entity) {
                    let cell = world_to_grid(transform.translation);
                    nav.grid.insert(cell, obstacle.cost);

                    if obstacle.cost == COST_BLOCKER {
                        events.write(PathBlockEvent { cell });
                    }
                }
            },
        ));

        // Реактивное обновление при удалении
        app.world_mut().spawn(Observer::new(
            |trigger: On<Remove, NavObstacle>,
             mut nav: ResMut<NavigationMap>,
             query: Query<&Transform, With<NavObstacle>>| {
                if let Ok(transform) = query.get(trigger.entity) {
                    let cell = world_to_grid(transform.translation);
                    nav.grid.remove(&cell);
                }
            },
        ));
    }
}
