use bevy::prelude::*;

/// Глобальные наборы систем для управления порядком выполнения (конвейер).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Сбор ввода (мышь, клавиатура)
    Input,
    /// Обработка игровой логики (ИИ, потребности, экономика)
    Logic,
    /// Обновление интерфейса и визуальных эффектов
    Visuals,
}

pub struct SetsPlugin;

impl Plugin for SetsPlugin {
    fn build(&self, app: &mut App) {
        // Настраиваем строгий порядок выполнения: Input -> Logic -> Visuals
        app.configure_sets(Update, (
            GameSet::Input,
            GameSet::Logic,
            GameSet::Visuals,
        ).chain());
    }
}
