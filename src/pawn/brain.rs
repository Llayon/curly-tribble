use super::{Hunger, Hungry, Settler};
use crate::economy::global::{EconomyCommandsExt, GlobalResources};
use crate::sets::GameSet;
use bevy::prelude::*;

use crate::map::navigation::{ComputingPath, NavigationCommandsExt, Path};
use crate::map::resources::BerryBush;
use crate::pawn::behaviors::{BehaviorExt, Gathering, Idle};
use crate::pawn::relations::Targeting;

pub struct BrainPlugin;

impl Plugin for BrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                think,
                find_resources,
                collect_berries,
                eat_from_stockpile,
                decide_construction,
            )
                .chain()
                .in_set(GameSet::Logic),
        );
    }
}

fn think(
    query: Query<(Entity, &Hunger), (With<Settler>, With<Hungry>, With<Idle>, Changed<Hunger>)>,
) {
    for (_entity, _hunger) in &query {
        // Здесь можно добавить сложную логику выбора целей
    }
}

fn find_resources(
    mut commands: Commands,
    idlers: Query<
        (Entity, &Hunger, &Transform),
        (
            With<Settler>,
            With<Idle>,
            Without<Targeting>,
            Without<ComputingPath>,
        ),
    >,
    bushes: Query<(Entity, &Transform), With<BerryBush>>,
    resources: Res<GlobalResources>,
) {
    for (settler, hunger, settler_transform) in &idlers {
        // Поселенец идет за едой если:
        // 1. Он голоден (>10%)
        // 2. В колонии мало еды (<200)
        let colony_needs_food = resources.food < 200.0;
        let is_hungry = hunger.value() > 10.0;

        if !is_hungry && !colony_needs_food {
            continue;
        }

        // Ищем БЛИЖАЙШИЙ куст
        if let Some((bush_entity, bush_transform)) = bushes.iter().min_by(|a, b| {
            let da = settler_transform
                .translation
                .distance_squared(a.1.translation);
            let db = settler_transform
                .translation
                .distance_squared(b.1.translation);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            // ВАЖНО: Сначала переключаем поведение, т.к. switch_behavior очищает Targeting
            commands.entity(settler).switch_behavior::<Gathering>();

            // Затем устанавливаем цель и приказ на перемещение (подходим на 1.1м для сетки 1х1)
            commands.entity(settler).insert(Targeting(bush_entity));
            commands.interact_with(settler, bush_transform.translation, 1.1);
        }
    }
}

fn collect_berries(
    mut commands: Commands,
    mut settlers: Query<
        (Entity, &mut Hunger, &Targeting, &Transform, Option<&Path>, Option<&ComputingPath>),
        (
            With<Settler>,
            With<Gathering>,
        ),
    >,
    mut bushes: Query<(&mut BerryBush, &Transform), With<BerryBush>>,
    time: Res<Time<Fixed>>,
    _resources: Res<GlobalResources>,
) {
    for (entity, mut hunger, target, settler_transform, path, computing) in &mut settlers {
        if let Ok((mut bush, bush_transform)) = bushes.get_mut(target.0) {
            // Используем 2D дистанцию (игнорируем Y) чтобы избежать проблем с вертикальностью
            let mut s_pos = settler_transform.translation;
            let mut b_pos = bush_transform.translation;
            s_pos.y = 0.0;
            b_pos.y = 0.0;
            let dist_2d = s_pos.distance(b_pos);

            // Если мы в радиусе сбора, МЫ ЕДИМ. Игнорируем статус навигации.
            // Это предотвращает зависание ИИ если путь не успел удалиться.
            if dist_2d < 1.5 {
                let amount = 5.0 * time.delta_secs();
                if amount > 0.0 {
                    bush.food_amount -= amount;
                    hunger.satisfy(amount * 10.0);
                    commands.add_food(amount);

                    if bush.food_amount <= 0.0 {
                        commands.entity(target.0).despawn();
                        commands.entity(entity).switch_behavior::<Idle>();
                    }
                }
            } else if path.is_none() && computing.is_none() {
                // Если мы НЕ в радиусе И НЕ идем — запрашиваем путь снова (подходим на 1.1м)
                commands.interact_with(entity, bush_transform.translation, 1.1);
            }
        } else {
            commands.entity(entity).switch_behavior::<Idle>();
        }
    }
}

fn eat_from_stockpile(
    mut query: Query<(Entity, &mut Hunger), (With<Settler>, With<Hungry>, With<Idle>)>,
    mut resources: ResMut<GlobalResources>,
    time: Res<Time<Fixed>>,
) {
    for (_entity, mut hunger) in &mut query {
        if resources.food > 0.1 {
            // Поедаем медленнее (1.0 ед/сек вместо 10.0)
            let amount = 1.0 * time.delta_secs();
            let consumed = amount.min(resources.food);

            if consumed > 0.0 {
                resources.food -= consumed;
                hunger.satisfy(consumed * 50.0); // Еда со склада ОЧЕНЬ сытная
            }
        }
    }
}
fn decide_construction(
    mut commands: Commands,
    resources: Res<GlobalResources>,
    idlers: Query<Entity, (With<Settler>, With<Idle>)>,
    time: Res<Time>,
    mut cooldown: Local<f32>,
) {
    // Уменьшаем кулдаун
    if *cooldown > 0.0 {
        *cooldown -= time.delta_secs();
        return;
    }

    // Повышаем порог строительства и добавляем кулдаун
    if resources.food > 100.0 {
        if let Some(_settler) = idlers.iter().next() {
            let pos = Vec3::new(5.0, 0.0, 5.0);
            commands.build_warding_stone(pos);
            *cooldown = 10.0; // Строим не чаще чем раз в 10 секунд
        }
    }
}
