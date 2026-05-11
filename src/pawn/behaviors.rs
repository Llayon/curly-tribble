use bevy::prelude::*;

pub struct BehaviorsPlugin;

impl Plugin for BehaviorsPlugin {
    fn build(&self, _app: &mut App) {
        // Здесь мы могли бы регистрировать типы для рефлексии, если понадобится
    }
}

// ============================================================================
// ПОВЕДЕНЧЕСКИЕ МАРКЕРЫ (ZST)
// ============================================================================

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Idle;

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Gathering;

#[derive(Component, Default, Debug, Clone, Copy)]
pub struct Eating;

pub type AllBehaviors = (Idle, Gathering, Eating);

// ============================================================================
// БЕЗОПАСНЫЙ ПЕРЕКЛЮЧАТЕЛЬ (Safe Switcher)
// ============================================================================

use crate::pawn::relations::Targeting;
use crate::map::navigation::{Path, ComputingPath};

pub trait BehaviorExt {
    fn switch_behavior<T: Component + Default>(&mut self);
}

impl BehaviorExt for EntityCommands<'_> {
    fn switch_behavior<T: Component + Default>(&mut self) {
        self.remove::<AllBehaviors>();
        // При смене задачи мы атомарно сбрасываем старую цель и пути,
        // чтобы граф всегда соответствовал текущему состоянию ИИ.
        self.remove::<Targeting>();
        self.remove::<Path>();
        self.remove::<ComputingPath>();
        self.insert(T::default());
    }
}
