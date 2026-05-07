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
            (think, find_resources, collect_berries, decide_construction)
                .chain()
                .in_set(GameSet::Logic),
        );
    }
}

fn think(query: Query<&Hunger, (With<Settler>, With<Hungry>, With<Idle>)>) {
    for hunger in &query {
        if hunger.value() > 50.0 {
            // Реакция на голод
        }
    }
}

fn find_resources(
    mut commands: Commands,
    idlers: Query<Entity, (With<Settler>, Added<Idle>, Without<Targeting>)>,
    bushes: Query<(Entity, &Transform), With<BerryBush>>,
) {
    for settler in &idlers {
        if let Some((bush_entity, bush_transform)) = bushes.iter().next() {
            // Устанавливаем цель
            commands.entity(settler).insert(Targeting(bush_entity));

            // ПРИКАЗ: Иди к цели (Зенитный Навигатор)
            commands.move_to(settler, bush_transform.translation);

            // Переключаемся в Gathering, но реально начнем собирать только по прибытии
            commands.entity(settler).switch_behavior::<Gathering>();
        }
    }
}

fn collect_berries(
    mut commands: Commands,
    mut settlers: Query<
        (Entity, &mut Hunger, &Targeting, &Transform),
        (
            With<Settler>,
            With<Gathering>,
            Without<Path>,
            Without<ComputingPath>,
        ),
    >,
    mut bushes: Query<(&mut BerryBush, &Transform), With<BerryBush>>,
    time: Res<Time<Fixed>>,
) {
    for (entity, mut hunger, target, settler_transform) in &mut settlers {
        if let Ok((mut bush, bush_transform)) = bushes.get_mut(target.0) {
            // Проверяем дистанцию (реалистичность)
            let dist = settler_transform
                .translation
                .distance(bush_transform.translation);
            if dist < 1.2 {
                // В РАДИУСЕ СБОРА
                let amount = 2.0 * time.delta_secs();
                bush.food_amount -= amount;
                hunger.satisfy(amount * 5.0);
                commands.add_food(amount);

                if bush.food_amount <= 0.0 {
                    commands.entity(target.0).despawn();
                    commands.entity(entity).switch_behavior::<Idle>();
                }
            } else {
                // ПОТЕРЯЛИСЬ ИЛИ ПУТЬ ПЕРЕГОРОЖЕН: запрашиваем путь снова
                commands.move_to(entity, bush_transform.translation);
            }
        } else {
            commands.entity(entity).switch_behavior::<Idle>();
        }
    }
}

fn decide_construction(
    mut commands: Commands,
    resources: Res<GlobalResources>,
    idlers: Query<Entity, (With<Settler>, With<Idle>)>,
) {
    if resources.food > 15.0 {
        if let Some(_settler) = idlers.iter().next() {
            // Строим в случайном месте неподалеку
            let pos = Vec3::new(3.0, 0.0, 3.0);
            commands.build_warding_stone(pos);
            // Чтобы не строить каждый кадр, можно добавить таймер или сбросить еду
        }
    }
}
