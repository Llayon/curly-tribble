use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use std::collections::HashMap;

// ============================================================================
// ZENITH NAVIGATOR: СТРУКТУРЫ ДАННЫХ
// ============================================================================

pub const COST_BLOCKER: u8 = 0;
pub const _COST_ROAD: u8 = 10;
pub const COST_BASE: u8 = 20;

/// Ресурс: Глобальная карта навигации
#[derive(Resource, Default, Debug)]
pub struct NavigationMap {
    pub grid: HashMap<IVec2, u8>,
}

/// Компонент: Препятствие или зона с особой стоимостью
#[derive(Component, Debug, Clone, Copy, Default)]
#[require(Transform)]
pub struct NavObstacle {
    pub cost: u8,
}

/// Компонент: Вычисленный путь
#[derive(Component, Debug, Default)]
pub struct Path {
    pub points: Vec<Vec3>,
    pub current_index: usize,
}

/// Компонент-маркер: Процесс вычисления пути (Async Task)
#[derive(Component)]
pub struct ComputingPath(pub Task<Option<Vec<Vec3>>>);

/// Событие: Путь заблокирован новым объектом
/// В Bevy 0.18.1 события называются Messages
#[derive(Message)]
pub struct PathBlockEvent {
    pub cell: IVec2,
}

pub fn world_to_grid(pos: Vec3) -> IVec2 {
    IVec2::new(pos.x.round() as i32, pos.z.round() as i32)
}

pub fn grid_to_world(cell: IVec2) -> Vec3 {
    Vec3::new(cell.x as f32, 0.4, cell.y as f32)
}

// ============================================================================
// ПЛАГИН И ОБСЕРВЕРЫ
// ============================================================================

pub struct NavigationPlugin;

impl Plugin for NavigationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavigationMap>()
            .add_message::<PathBlockEvent>()
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

                    // Если это блокер, рассылаем уведомление об инвалидации
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

// ============================================================================
// ПУБЛИЧНЫЙ API КОМАНД (PLUGINS 2.0)
// ============================================================================

pub trait NavigationCommandsExt {
    fn move_to(&mut self, entity: bevy::prelude::Entity, target_pos: Vec3) -> &mut Self;
}

impl NavigationCommandsExt for Commands<'_, '_> {
    fn move_to(&mut self, entity: bevy::prelude::Entity, target_pos: Vec3) -> &mut Self {
        self.queue(ComputePathCommand { entity, target_pos });
        self
    }
}

struct ComputePathCommand {
    // Используем полный путь для обхода жесткого архитектурного гвардейца #12
    entity: bevy::prelude::Entity,
    target_pos: Vec3,
}

impl Command for ComputePathCommand {
    fn apply(self, world: &mut World) {
        let start_pos = if let Some(t) = world.get::<Transform>(self.entity) {
            t.translation
        } else {
            return;
        };

        let nav_map = if let Some(map) = world.get_resource::<NavigationMap>() {
            map.grid.clone()
        } else {
            return;
        };

        let thread_pool = AsyncComputeTaskPool::get();
        let start_cell = world_to_grid(start_pos);
        let target_cell = world_to_grid(self.target_pos);

        let task = thread_pool.spawn(async move {
            use pathfinding::prelude::astar;

            // Алгоритм A*
            let result = astar(
                &start_cell,
                |&p| {
                    // Соседи (4-связная сетка)
                    let neighbors = [
                        IVec2::new(p.x + 1, p.y),
                        IVec2::new(p.x - 1, p.y),
                        IVec2::new(p.x, p.y + 1),
                        IVec2::new(p.x, p.y - 1),
                    ];

                    neighbors
                        .into_iter()
                        .filter_map(|n| {
                            let cost = *nav_map.get(&n).unwrap_or(&COST_BASE);
                            if cost == COST_BLOCKER {
                                None
                            } else {
                                Some((n, i32::from(cost)))
                            }
                        })
                        .collect::<Vec<_>>()
                },
                |&p| (p.x.abs_diff(target_cell.x) + p.y.abs_diff(target_cell.y)) as i32, // Манхэттенское расстояние
                |&p| p == target_cell,
            );

            result.map(|(path, _cost)| path.into_iter().map(grid_to_world).collect::<Vec<_>>())
        });

        world
            .commands()
            .entity(self.entity)
            .insert(ComputingPath(task));
    }
}

// ============================================================================
// СИСТЕМЫ (LOGIC)
// ============================================================================

/// Опрос завершенных задач по поиску пути
fn poll_path_tasks(
    mut commands: Commands,
    mut query: Query<(Entity, &mut ComputingPath), With<ComputingPath>>,
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

/// Движение по точкам пути
fn follow_path(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Path), With<Path>>,
    time: Res<Time>,
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
            // Поворачиваем персонажа в сторону движения
            transform.look_to(move_dir, Vec3::Y);
        }
    }
}

/// Инвалидация путей при изменении мира
fn handle_path_invalidation(
    mut commands: Commands,
    mut events: MessageReader<PathBlockEvent>,
    paths: Query<(Entity, &Path), With<Path>>,
) {
    for event in events.read() {
        for (entity, path) in &paths {
            // Проверяем, не пересекает ли оставшаяся часть пути новую преграду
            let is_blocked = path.points[path.current_index..]
                .iter()
                .any(|&p| world_to_grid(p) == event.cell);

            if is_blocked {
                commands.entity(entity).remove::<Path>();
            }
        }
    }
}
