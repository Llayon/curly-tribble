use crate::economy::GlobalResources;
use crate::game_state::{CurrentTool, EditorPhase, GameState, ShapeTool};
use crate::map::{MapData, TerrainType, ForestType, PoiType};
use crate::sets::{GameSet, StartupSet};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts, EguiPrimaryContextPass};

pub mod details;
pub mod logs;
pub mod resources;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            resources::ResourceUiPlugin,
            details::DetailUiPlugin,
            logs::GameLogPlugin,
        ));

        app.add_systems(Startup, setup_ui.in_set(StartupSet::SpawnEntities))
            .add_systems(
                Update,
                (
                    resources::update_resource_ui
                        .run_if(resource_changed::<GlobalResources>)
                        .in_set(GameSet::Visuals),
                    details::update_settler_detail_ui.in_set(GameSet::Visuals),
                ),
            );

        // В Bevy 0.18.1 / bevy_egui 0.39 используем специальный Schedule для Egui
        app.add_systems(
            EguiPrimaryContextPass,
            editor_phase_ui.run_if(in_state(GameState::Playing)),
        );
    }
}

fn editor_phase_ui(
    mut contexts: EguiContexts,
    current_phase: Res<State<EditorPhase>>,
    mut next_phase: ResMut<NextState<EditorPhase>>,
    mut current_tool: ResMut<CurrentTool>,
    mut faction_manager: ResMut<crate::game_state::FactionManager>,
    map_data: Res<MapData>,
    mut terrain_config: ResMut<crate::map::terrain_gen::TerrainConfig>,
    mut ev_rebuild: MessageWriter<crate::map::RebuildMeshEvent>,
) {
    let ctx = match contexts.ctx_mut().ok() {
        Some(ctx) => ctx,
        None => return,
    };

    let is_valid = map_data.validation_errors.is_empty();

    // --- TOP PANEL: Global Filters & View Modes ---
    egui::TopBottomPanel::top("top_filter_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("VIEW:");
            if ui
                .selectable_label(terrain_config.show_forests, "🌲 Forests")
                .clicked()
            {
                terrain_config.show_forests = !terrain_config.show_forests;
                ev_rebuild.write(crate::map::RebuildMeshEvent);
            }
            if ui
                .selectable_label(terrain_config.show_factions, "🚩 Factions")
                .clicked()
            {
                terrain_config.show_factions = !terrain_config.show_factions;
                ev_rebuild.write(crate::map::RebuildMeshEvent);
            }
            if ui
                .selectable_label(terrain_config.show_cliffs, "📐 Cliffs")
                .clicked()
            {
                terrain_config.show_cliffs = !terrain_config.show_cliffs;
                // Cliffs are gizmos, don't strictly need mesh rebuild, but good for consistency
            }
            if ui
                .selectable_label(terrain_config.show_build_area, "🧱 Build Area")
                .clicked()
            {
                terrain_config.show_build_area = !terrain_config.show_build_area;
                ev_rebuild.write(crate::map::RebuildMeshEvent);
            }
            ui.separator();
            ui.label(format!("Phase: {:?}", current_phase.get()));
        });
    });

    // --- BOTTOM PANEL: Phase Timeline ---
    egui::TopBottomPanel::bottom("phase_timeline").show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            let phases = [
                (EditorPhase::Shape, "1. Shape"),
                (EditorPhase::Factions, "2. Factions"),
                (EditorPhase::Landscape, "3. Landscape"),
                (EditorPhase::Sediments, "4. Sediments"),
                (EditorPhase::NPCs, "5. NPCs"),
                (EditorPhase::Height3D, "6. Height3D"),
            ];

            let current_idx = phases
                .iter()
                .position(|(p, _)| p == current_phase.get())
                .unwrap_or(0);

            for (idx, (phase, label)) in phases.into_iter().enumerate() {
                let is_current = *current_phase.get() == phase;
                let is_physically_reachable = idx <= current_idx + 1;
                let needs_validation = idx > 1;
                let can_click = is_physically_reachable && (!needs_validation || is_valid);

                ui.add_enabled_ui(can_click, |ui| {
                    if ui.selectable_label(is_current, label).clicked() {
                        next_phase.set(phase);
                    }
                });
                if idx < phases.len() - 1 {
                    ui.label("→");
                }
            }
        });
    });

    // --- LEFT PANEL: Context-Sensitive Tools (Brushes) ---
    egui::SidePanel::left("tool_sidebar")
        .default_width(120.0)
        .show(ctx, |ui| {
            ui.heading("Tools");
            ui.separator();

            match current_phase.get() {
                EditorPhase::Shape => {
                    ui.label("Island Shape:");
                    if ui
                        .selectable_label(current_tool.shape == ShapeTool::None, "None")
                        .clicked()
                    {
                        current_tool.shape = ShapeTool::None;
                    }
                    if ui
                        .selectable_label(current_tool.shape == ShapeTool::Ocean, "Ocean")
                        .clicked()
                    {
                        current_tool.shape = ShapeTool::Ocean;
                    }
                }
                EditorPhase::Factions => {
                    ui.label("Faction Painting:");
                    if ui
                        .selectable_label(
                            current_tool.faction == crate::game_state::FactionTool::None,
                            "None",
                        )
                        .clicked()
                    {
                        current_tool.faction = crate::game_state::FactionTool::None;
                    }
                    if ui
                        .selectable_label(
                            current_tool.faction == crate::game_state::FactionTool::Brush,
                            "Brush",
                        )
                        .clicked()
                    {
                        current_tool.faction = crate::game_state::FactionTool::Brush;
                    }
                }
                EditorPhase::Landscape => {
                    ui.label("Landscape Brushes:");
                    let tools = [
                        (crate::game_state::LandscapeTool::None, "None"),
                        (crate::game_state::LandscapeTool::Mountain, "Mountain"),
                        (crate::game_state::LandscapeTool::Lake, "Lake"),
                        (crate::game_state::LandscapeTool::River, "River"),
                        (crate::game_state::LandscapeTool::Plateau, "Plateau"),
                        (crate::game_state::LandscapeTool::Cliff, "Cliff"),
                    ];
                    for (tool, label) in tools {
                        if ui
                            .selectable_label(current_tool.landscape == tool, label)
                            .clicked()
                        {
                            current_tool.landscape = tool;
                        }
                    }
                }
                EditorPhase::Sediments => {
                    ui.label("Sediment Tool:");
                    ui.checkbox(&mut current_tool.active_sediment_tool, "Active");
                    egui::ComboBox::from_id_salt("sediment_type")
                        .selected_text(format!("{:?}", current_tool.sediment))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Dirt,
                                "Dirt",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Dusty,
                                "Dusty",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Fertile,
                                "Fertile",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Mossy,
                                "Mossy",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Steppe,
                                "Steppe",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Stony,
                                "Stony",
                            );
                            ui.selectable_value(
                                &mut current_tool.sediment,
                                TerrainType::Swamp,
                                "Swamp",
                            );
                        });

                    ui.separator();
                    ui.label("Forest Tool:");
                    ui.checkbox(&mut current_tool.active_forest_tool, "Active");
                    egui::ComboBox::from_id_salt("forest_type")
                        .selected_text(format!("{:?}", current_tool.forest_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut current_tool.forest_type,
                                ForestType::None,
                                "None",
                            );
                            ui.selectable_value(
                                &mut current_tool.forest_type,
                                ForestType::Deciduous,
                                "Deciduous",
                            );
                            ui.selectable_value(
                                &mut current_tool.forest_type,
                                ForestType::Coniferous,
                                "Coniferous",
                            );
                        });
                    ui.add(
                        egui::Slider::new(&mut current_tool.forest_density, 0.0..=1.0)
                            .text("Density"),
                    );
                }
                EditorPhase::NPCs => {
                    ui.label("NPC Tools:");
                    let tools = [
                        (crate::game_state::NpcTool::None, "None"),
                        (crate::game_state::NpcTool::SpawnPoi, "Spawn POI"),
                        (
                            crate::game_state::NpcTool::SpawnEnemyCamp,
                            "Spawn Enemy Camp",
                        ),
                        (crate::game_state::NpcTool::Delete, "Delete"),
                    ];
                    for (tool, label) in tools {
                        if ui
                            .selectable_label(current_tool.npc == tool, label)
                            .clicked()
                        {
                            current_tool.npc = tool;
                        }
                    }
                }
                _ => {
                    ui.label("No tools for this phase.");
                }
            }
        });

    // --- RIGHT PANEL: Hierarchy & Properties ---
    egui::SidePanel::right("inspector_sidebar")
        .default_width(250.0)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                if !is_valid {
                    ui.collapsing("⚠️ Validation Errors", |ui| {
                        for err in &map_data.validation_errors {
                            ui.colored_label(egui::Color32::RED, format!("• {}", err));
                        }
                    });
                }

                // Faction Hierarchy (Only in Factions and NPCs phases)
                if *current_phase.get() == EditorPhase::Factions
                    || *current_phase.get() == EditorPhase::NPCs
                {
                    ui.collapsing("🚩 Faction Hierarchy", |ui| {
                        let mut to_remove = None;
                        let mut to_select = None;
                        for (idx, faction) in faction_manager.factions.iter().enumerate() {
                            let is_selected = faction_manager.selected_faction == Some(faction.id);
                            ui.horizontal(|ui| {
                                if ui.selectable_label(is_selected, &faction.name).clicked() {
                                    to_select = Some(faction.id);
                                }
                                if faction.faction_type != crate::game_state::FactionType::Player {
                                    if ui.button("🗑").clicked() {
                                        to_remove = Some(idx);
                                    }
                                }
                            });
                        }
                        if let Some(id) = to_select {
                            faction_manager.selected_faction = Some(id);
                        }
                        if let Some(idx) = to_remove {
                            faction_manager.factions.remove(idx);
                            faction_manager.selected_faction = None;
                        }
                        if ui.button("Add Neutral Faction").clicked() {
                            let next_id = faction_manager
                                .factions
                                .iter()
                                .map(|f| f.id)
                                .max()
                                .unwrap_or(0)
                                + 1;
                            faction_manager.factions.push(crate::game_state::Faction {
                                id: next_id,
                                name: format!("Faction {}", next_id),
                                faction_type: crate::game_state::FactionType::Neutral,
                                color: Color::srgb(rand::random(), rand::random(), rand::random()),
                                economy_focus: "None".to_string(),
                            });
                        }
                    });
                }

                // Selection Properties
                ui.collapsing("🔍 Selection Properties", |ui| {
                    if let Some(selected_id) = faction_manager.selected_faction {
                        if let Some(faction) = faction_manager
                            .factions
                            .iter_mut()
                            .find(|f| f.id == selected_id)
                        {
                            ui.label(format!("Editing: {}", faction.name));
                            ui.text_edit_singleline(&mut faction.name);
                            ui.horizontal(|ui| {
                                ui.label("Type:");
                                egui::ComboBox::from_id_salt("faction_type_prop")
                                    .selected_text(format!("{:?}", faction.faction_type))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut faction.faction_type,
                                            crate::game_state::FactionType::Player,
                                            "Player",
                                        );
                                        ui.selectable_value(
                                            &mut faction.faction_type,
                                            crate::game_state::FactionType::Neutral,
                                            "Neutral",
                                        );
                                        ui.selectable_value(
                                            &mut faction.faction_type,
                                            crate::game_state::FactionType::Enemy,
                                            "Enemy",
                                        );
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Economy:");
                                egui::ComboBox::from_id_salt("economy_focus_prop")
                                    .selected_text(&faction.economy_focus)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut faction.economy_focus,
                                            "None".to_string(),
                                            "None",
                                        );
                                        ui.selectable_value(
                                            &mut faction.economy_focus,
                                            "Mining".to_string(),
                                            "Mining",
                                        );
                                        ui.selectable_value(
                                            &mut faction.economy_focus,
                                            "Farming".to_string(),
                                            "Farming",
                                        );
                                        ui.selectable_value(
                                            &mut faction.economy_focus,
                                            "Woodcutting".to_string(),
                                            "Woodcutting",
                                        );
                                    });
                            });
                        }
                    } else if current_tool.npc == crate::game_state::NpcTool::SpawnEnemyCamp {
                        ui.label("Enemy Camp Settings:");
                        ui.add(
                            egui::Slider::new(&mut current_tool.camp_difficulty, 0.0..=1.0)
                                .text("Difficulty"),
                        );
                        ui.add(
                            egui::Slider::new(&mut current_tool.camp_power, 10..=1000)
                                .text("Combat Power"),
                        );
                    } else if current_tool.npc == crate::game_state::NpcTool::SpawnPoi {
                        ui.label("POI Settings:");
                        egui::ComboBox::from_id_salt("poi_type_prop")
                            .selected_text(format!("{:?}", current_tool.poi_type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut current_tool.poi_type,
                                    PoiType::TradePost,
                                    "TradePost",
                                );
                                ui.selectable_value(
                                    &mut current_tool.poi_type,
                                    PoiType::Ruins,
                                    "Ruins",
                                );
                                ui.selectable_value(
                                    &mut current_tool.poi_type,
                                    PoiType::Shrine,
                                    "Shrine",
                                );
                                ui.selectable_value(
                                    &mut current_tool.poi_type,
                                    PoiType::Treasure,
                                    "Treasure",
                                );
                            });
                    } else {
                        ui.label("No selection.");
                    }
                });
            });
        });
}

fn setup_ui(mut commands: Commands) {
    // В Bevy 0.18.1 отдельная Camera2d для UI не нужна!
    // UI автоматически отрисовывается поверх основной 3D камеры.
    // Удаление Camera2d предотвращает "Double Camera Trap" (затирание 3D мира серым фоном).

    // 1. Top-left: Global Resources
    let mut resources_node = commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
    ));
    resources::setup_resource_ui(&mut resources_node);

    // 2. Bottom-right: Settler Details
    let mut details_node = commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(15.0)),
            min_width: Val::Px(250.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.1, 0.2, 0.9)),
    ));
    details::setup_detail_ui(&mut details_node);

    // 3. Bottom-left: Game Log
    let mut log_node = commands.spawn(Node {
        position_type: PositionType::Absolute,
        bottom: Val::Px(10.0),
        left: Val::Px(10.0),
        ..default()
    });
    logs::setup_log_ui(&mut log_node);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ui_exists() {
        let mut app = App::new();
        app.init_resource::<GlobalResources>();
        app.add_message::<crate::events::GameLogMessage>();
        app.add_plugins(UiPlugin);

        app.finish();
        app.cleanup();
        app.update();

        // Проверяем, что узлы интерфейса созданы (ищем по фоновому цвету или нодам)
        let mut query = app.world_mut().query::<&Node>();
        assert!(
            query.iter(app.world()).count() > 0,
            "UI Nodes should be spawned"
        );
    }
}
