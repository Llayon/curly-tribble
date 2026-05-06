use bevy::prelude::*;

// ============================================================================
// ТИПИЗИРОВАННЫЕ ОТНОШЕНИЯ (RFC 76 / Bevy 0.18.1)
// ============================================================================

pub struct RelationsPlugin;

impl Plugin for RelationsPlugin {
    fn build(&self, _app: &mut App) {}
}

/// Отношение: "Целится на / Сфокусирован на"
/// ВАЖНО: Поскольку это одиночный компонент, Bevy гарантирует эксклюзивность (одна цель).
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[relationship(relationship_target = TargetedBy)]
pub struct Targeting(pub Entity);

/// Обратная ссылка для Targeting
#[derive(Component, Debug, Clone, Reflect)]
#[relationship_target(relationship = Targeting)]
pub struct TargetedBy(Vec<Entity>);

impl TargetedBy {
    pub fn entities(&self) -> &[Entity] {
        &self.0
    }
}

/// Отношение: "Находится в зоне освещения"
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[relationship(relationship_target = Illuminating)]
pub struct InsideLightOf(pub Entity);

/// Обратная ссылка для InsideLightOf
#[derive(Component, Debug, Clone, Reflect)]
#[relationship_target(relationship = InsideLightOf)]
pub struct Illuminating(Vec<Entity>);

impl Illuminating {
    pub fn entities(&self) -> &[Entity] {
        &self.0
    }
}
