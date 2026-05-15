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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum ShapeTool {
    #[default]
    None,
    Ocean,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct CurrentTool {
    pub shape: ShapeTool,
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_state::<EditorPhase>()
            .init_resource::<CurrentTool>()
            .register_type::<EditorPhase>()
            .register_type::<ShapeTool>()
            .register_type::<CurrentTool>()
            .add_systems(PostStartup, start_game);
    }
}

fn start_game(mut next_state: ResMut<NextState<GameState>>) {
    info!("Transitioning from Loading to Playing state");
    next_state.set(GameState::Playing);
}
