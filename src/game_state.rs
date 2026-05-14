use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Copy, Reflect)]
pub enum GameState {
    #[default]
    Loading,
    Playing,
    #[allow(dead_code)]
    Paused,
}

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Reflect)]
pub enum EditorPhase {
    #[default]
    Shape,
    Sediments,
    Flora,
    Height3D,
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_state::<EditorPhase>()
            .register_type::<EditorPhase>()
            .add_systems(PostStartup, start_game);
    }
}

fn start_game(mut next_state: ResMut<NextState<GameState>>) {
    info!("Transitioning from Loading to Playing state");
    next_state.set(GameState::Playing);
}
