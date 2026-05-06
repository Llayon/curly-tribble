use super::{Hunger, Hungry, Settler};
use crate::economy::global::{EconomyCommandsExt, GlobalResources};
use crate::sets::GameSet;
use bevy::prelude::*;

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
            // Реакция на голод будет добавлена позже
        }
    }
}

fn find_resources(
    mut commands: Commands,
    idlers: Query<Entity, (With<Settler>, Added<Idle>, Without<Targeting>)>,
    bushes: Query<Entity, With<BerryBush>>,
) {
    for settler in &idlers {
        if let Some(bush) = bushes.iter().next() {
            commands.entity(settler).insert(Targeting(bush));
            commands.entity(settler).switch_behavior::<Gathering>();
        }
    }
}

fn collect_berries(
    mut commands: Commands,
    mut settlers: Query<(Entity, &mut Hunger, &Targeting), (With<Settler>, With<Gathering>)>,
    mut bushes: Query<&mut BerryBush, With<BerryBush>>,
    time: Res<Time<Fixed>>,
) {
    for (entity, mut hunger, target) in &mut settlers {
        if let Ok(mut bush) = bushes.get_mut(target.0) {
            let amount = 2.0 * time.delta_secs();
            bush.food_amount -= amount;
            hunger.satisfy(amount * 5.0);
            commands.add_food(amount);

            if bush.food_amount <= 0.0 {
                commands.entity(target.0).despawn();
                commands.entity(entity).switch_behavior::<Idle>();
            }
        } else {
            commands.entity(entity).switch_behavior::<Idle>();
        }
    }
}

fn decide_construction(
    mut commands: Commands,
    resources: Res<GlobalResources>,
    idlers: Query<&Transform, (With<Settler>, With<Idle>)>,
) {
    // Если еды много, поселенец решает расширить безопасную зону
    if resources.food > 15.0 {
        if let Some(transform) = idlers.iter().next() {
            // Строим камень чуть в стороне
            let pos = transform.translation + Vec3::new(2.0, 0.0, 2.0);
            commands.build_warding_stone(pos);
        }
    }
}
