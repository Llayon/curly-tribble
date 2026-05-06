use bevy::prelude::*;
use crate::game_state::GameState;

/// Наборы систем для игрового цикла (Update).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameSet {
    Input,
    Logic,
    Visuals,
}

/// Наборы систем для инициализации (Startup).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum StartupSet {
    /// Загрузка ресурсов (меши, материалы)
    LoadAssets,
    /// Создание сущностей (карта, игроки, свет)
    SpawnEntities,
}

pub struct SetsPlugin;

impl Plugin for SetsPlugin {
    fn build(&self, app: &mut App) {
        // 1. Конфигурация для расписания Update (Рендеринг/Ввод)
        app.configure_sets(Update, (
            GameSet::Input,
            GameSet::Logic,
            GameSet::Visuals,
        ).chain());
        
        app.configure_sets(Update, (
            GameSet::Input,
            GameSet::Logic,
        ).run_if(in_state(GameState::Playing)));

        // 2. Конфигурация для расписания FixedUpdate (Симуляция)
        app.configure_sets(FixedUpdate, (
            GameSet::Input,
            GameSet::Logic,
            GameSet::Visuals,
        ).chain());

        app.configure_sets(FixedUpdate, (
            GameSet::Input,
            GameSet::Logic,
        ).run_if(in_state(GameState::Playing)));

        // 3. Упорядочиваем фазу запуска (Startup)
        app.configure_sets(Startup, (
            StartupSet::LoadAssets,
            StartupSet::SpawnEntities,
        ).chain());
    }
}
