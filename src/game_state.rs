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
    Factions,
    Landscape,
    Sediments,
    Height3D,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum FactionType {
    #[default]
    Player,
    Neutral,
    Enemy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum ShapeTool {
    #[default]
    None,
    Ocean,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum FactionTool {
    #[default]
    None,
    Brush,
}

#[derive(Debug, Clone, Reflect)]
pub struct Faction {
    pub id: u32,
    pub name: String,
    pub faction_type: FactionType,
    pub color: Color,
    pub economy_focus: String,
}

#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct FactionManager {
    pub factions: Vec<Faction>,
    pub selected_faction: Option<u32>,
}

impl Default for FactionManager {
    fn default() -> Self {
        Self {
            factions: vec![Faction {
                id: 1,
                name: "Player".to_string(),
                faction_type: FactionType::Player,
                color: Color::srgb(0.6, 0.4, 0.2), // Brown-ish
                economy_focus: "None".to_string(),
            }],
            selected_faction: Some(1),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum LandscapeTool {
    #[default]
    None,
    Mountain,
    Lake,
    River,
    Plateau,
    Cliff,
}

#[derive(Resource, Default, Reflect)]
#[reflect(Resource)]
pub struct CurrentTool {
    pub shape: ShapeTool,
    pub faction: FactionTool,
    pub landscape: LandscapeTool,
    pub sediment: crate::map::zoning::TerrainType,
    pub forest_type: crate::map::zoning::ForestType,
    pub forest_density: f32,
    pub active_sediment_tool: bool,
    pub active_forest_tool: bool,
}

pub struct GameStatePlugin;

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .init_state::<EditorPhase>()
            .init_resource::<CurrentTool>()
            .init_resource::<FactionManager>()
            .register_type::<EditorPhase>()
            .register_type::<FactionType>()
            .register_type::<ShapeTool>()
            .register_type::<FactionTool>()
            .register_type::<LandscapeTool>()
            .register_type::<CurrentTool>()
            .register_type::<Faction>()
            .register_type::<FactionManager>()
            .add_systems(PostStartup, start_game);
    }
}

fn start_game(mut next_state: ResMut<NextState<GameState>>) {
    info!("Transitioning from Loading to Playing state");
    next_state.set(GameState::Playing);
}
