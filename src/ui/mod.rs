use crate::economy::GlobalResources;
use crate::game_state::{CurrentTool, EditorPhase, GameState, ShapeTool};
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
    map_data: Res<crate::map::zoning::MapData>,
) {
    let ctx = match contexts.ctx_mut().ok() {
        Some(ctx) => ctx,
        None => return,
    };

    egui::Window::new("Editor Phases")
        .id(egui::Id::new("stable_editor_phases_window"))
        .default_pos(egui::pos2(150.0, 150.0))
        .default_size(egui::vec2(320.0, 100.0))
        .movable(true)
        .resizable(true)
        .title_bar(true)
        .collapsible(true)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label("Phases:");

                let is_valid = map_data.validation_errors.is_empty();

                ui.horizontal_wrapped(|ui| {
                    let phases = [
                        EditorPhase::Shape,
                        EditorPhase::Factions,
                        EditorPhase::Landscape,
                        EditorPhase::Sediments,
                        EditorPhase::Height3D,
                    ];

                    for phase in phases {
                        let label = format!("{:?}", phase);
                        let is_current = *current_phase.get() == phase;
                        
                        // Фазы после Factions требуют валидации острова
                        let needs_validation = match phase {
                            EditorPhase::Shape | EditorPhase::Factions => false,
                            _ => true,
                        };
                        
                        let can_click = is_current || !needs_validation || is_valid;

                        ui.add_enabled_ui(can_click, |ui| {
                            if ui.selectable_label(is_current, label).clicked() {
                                next_phase.set(phase);
                            }
                        });
                    }
                });

                if !is_valid {
                    ui.separator();
                    for err in &map_data.validation_errors {
                        ui.colored_label(egui::Color32::RED, format!("⚠️ {}", err));
                    }
                }

                if *current_phase.get() == EditorPhase::Shape {
                    ui.separator();
                    ui.label("Shape Tools:");
                    ui.horizontal(|ui| {
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
                    });
                }

                if *current_phase.get() == EditorPhase::Factions {
                    ui.separator();
                    ui.label("Faction Tools:");
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(current_tool.faction == crate::game_state::FactionTool::None, "None")
                            .clicked()
                        {
                            current_tool.faction = crate::game_state::FactionTool::None;
                        }
                        if ui
                            .selectable_label(current_tool.faction == crate::game_state::FactionTool::Brush, "Brush")
                            .clicked()
                        {
                            current_tool.faction = crate::game_state::FactionTool::Brush;
                        }
                    });

                    ui.separator();
                    ui.label("Faction Hierarchy:");
                    
                    let mut to_remove = None;
                    let mut to_select = None;

                    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
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
                    });

                    if let Some(id) = to_select {
                        faction_manager.selected_faction = Some(id);
                    }

                    if let Some(idx) = to_remove {
                        faction_manager.factions.remove(idx);
                        faction_manager.selected_faction = None;
                    }

                    if ui.button("Add Neutral Faction").clicked() {
                        let next_id = faction_manager.factions.iter().map(|f| f.id).max().unwrap_or(0) + 1;
                        faction_manager.factions.push(crate::game_state::Faction {
                            id: next_id,
                            name: format!("Faction {}", next_id),
                            faction_type: crate::game_state::FactionType::Neutral,
                            color: Color::srgb(rand::random(), rand::random(), rand::random()),
                            economy_focus: "None".to_string(),
                        });
                    }

                    if let Some(selected_id) = faction_manager.selected_faction {
                        if let Some(faction) = faction_manager.factions.iter_mut().find(|f| f.id == selected_id) {
                            ui.separator();
                            ui.label("Faction Properties:");
                            ui.text_edit_singleline(&mut faction.name);
                            
                            ui.horizontal(|ui| {
                                ui.label("Type:");
                                egui::ComboBox::from_id_salt("faction_type")
                                    .selected_text(format!("{:?}", faction.faction_type))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut faction.faction_type, crate::game_state::FactionType::Player, "Player");
                                        ui.selectable_value(&mut faction.faction_type, crate::game_state::FactionType::Neutral, "Neutral");
                                        ui.selectable_value(&mut faction.faction_type, crate::game_state::FactionType::Enemy, "Enemy");
                                    });
                            });

                            ui.horizontal(|ui| {
                                ui.label("Economy:");
                                egui::ComboBox::from_id_salt("economy_focus")
                                    .selected_text(&faction.economy_focus)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut faction.economy_focus, "None".to_string(), "None");
                                        ui.selectable_value(&mut faction.economy_focus, "Mining".to_string(), "Mining");
                                        ui.selectable_value(&mut faction.economy_focus, "Farming".to_string(), "Farming");
                                        ui.selectable_value(&mut faction.economy_focus, "Woodcutting".to_string(), "Woodcutting");
                                    });
                            });
                        }
                    }
                }
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
