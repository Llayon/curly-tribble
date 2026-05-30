use bevy::prelude::*;

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Copy, Reflect)]
pub enum GameState {
    #[default]
    Loading,
    Playing,
    #[allow(dead_code)]
    Paused,
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Copy, Reflect, PartialOrd, Ord)]
pub enum EditorPhase {
    #[default]
    Shape,
    Factions,
    Landscape,
    Sediments,
    NPCs,
    Plants,
    Treasures,
    Artifacts,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum NpcTool {
    #[default]
    None,
    SpawnPoi,
    SpawnEnemyCamp,
    Move,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
pub enum TreasureToolMode {
    #[default]
    SpawnVisible,
    SpawnHidden,
    Link,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default, Reflect)]
#[reflect(Component)]
pub struct Selected;

#[derive(Resource, Reflect, Clone, Default)]
#[reflect(Resource)]
pub struct ArtifactToolState {
    pub selected_artifact: Option<crate::map::TargetEntity>,
    pub placing_on_ground: bool,
}

#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct CurrentTool {
    pub shape: ShapeTool,
    pub faction: FactionTool,
    pub landscape: LandscapeTool,
    pub sediment: crate::map::TerrainType,
    pub forest_type: crate::map::ForestType,
    pub forest_density: f32,
    pub active_sediment_tool: bool,
    pub active_forest_tool: bool,
    pub npc: NpcTool,
    pub poi_type: crate::map::PoiType,
    pub camp_difficulty: f32,
    pub camp_power: u32,
    pub bio_resource: crate::map::DepositType,
    pub bio_amount: u32,
    pub bio_brush_size: u32,
    pub treasure_mode: TreasureToolMode,
}

impl Default for CurrentTool {
    fn default() -> Self {
        Self {
            shape: ShapeTool::None,
            faction: FactionTool::None,
            landscape: LandscapeTool::None,
            sediment: crate::map::TerrainType::Dirt,
            forest_type: crate::map::ForestType::None,
            forest_density: 0.5,
            active_sediment_tool: false,
            active_forest_tool: false,
            npc: NpcTool::None,
            poi_type: crate::map::PoiType::TradePost,
            camp_difficulty: 0.5,
            camp_power: 100,
            bio_resource: crate::map::DepositType::Rabbit,
            bio_amount: 10,
            bio_brush_size: 1,
            treasure_mode: TreasureToolMode::default(),
        }
    }
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
            .register_type::<NpcTool>()
            .register_type::<TreasureToolMode>()
            .register_type::<Selected>()
            .register_type::<CurrentTool>()
            .register_type::<Faction>()
            .register_type::<FactionManager>()
            .register_type::<crate::map::DepositType>()
            .register_type::<crate::map::TerrainType>()
            .register_type::<crate::map::ForestType>()
            .init_resource::<ArtifactToolState>()
            .register_type::<ArtifactToolState>()
            .add_systems(PostStartup, start_game);
    }
}

fn start_game(mut next_state: ResMut<NextState<GameState>>) {
    info!("Transitioning from Loading to Playing state");
    next_state.set(GameState::Playing);
}
