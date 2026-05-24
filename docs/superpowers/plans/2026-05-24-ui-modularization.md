# UI Modularization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor `src/ui/mod.rs` to bring it below 300 lines (currently 544) by modularizing editor UI components into a new `panels` submodule.

**Architecture:** Decompose the massive `editor_phase_ui` system into smaller, focused functions located in `src/ui/panels/`. `src/ui/mod.rs` will remain as the high-level dispatcher.

**Tech Stack:** Bevy 0.18.1, bevy_egui, egui.

---

### Task 1: Create Panels Module and Top Bar

**Files:**
- Create: `src/ui/panels/mod.rs`
- Create: `src/ui/panels/top_bar.rs`

- [ ] **Step 1: Create `src/ui/panels/mod.rs`**
```rust
pub mod top_bar;
pub mod bottom_bar;
pub mod tools;
pub mod inspector;
```

- [ ] **Step 2: Create `src/ui/panels/top_bar.rs`**
```rust
use bevy::prelude::*;
use bevy_egui::egui;
use crate::game_state::EditorPhase;
use crate::map::terrain_gen::TerrainConfig;
use crate::map::RebuildMeshEvent;

pub fn show_top_bar(
    ctx: &egui::Context,
    current_phase: &EditorPhase,
    terrain_config: &mut TerrainConfig,
    ev_rebuild: &mut MessageWriter<RebuildMeshEvent>,
) {
    egui::TopBottomPanel::top("top_filter_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.label("VIEW:");
            if ui
                .selectable_label(terrain_config.show_forests, "🌲 Forests")
                .clicked()
            {
                terrain_config.show_forests = !terrain_config.show_forests;
                ev_rebuild.write(RebuildMeshEvent);
            }
            if ui
                .selectable_label(terrain_config.show_factions, "🚩 Factions")
                .clicked()
            {
                terrain_config.show_factions = !terrain_config.show_factions;
                ev_rebuild.write(RebuildMeshEvent);
            }
            if ui
                .selectable_label(terrain_config.show_cliffs, "📐 Cliffs")
                .clicked()
            {
                terrain_config.show_cliffs = !terrain_config.show_cliffs;
            }
            if ui
                .selectable_label(terrain_config.show_build_area, "🧱 Build Area")
                .clicked()
            {
                terrain_config.show_build_area = !terrain_config.show_build_area;
                ev_rebuild.write(RebuildMeshEvent);
            }
            ui.separator();
            ui.label(format!("Phase: {:?}", current_phase));
        });
    });
}
```

- [ ] **Step 3: Commit Task 1**
```bash
git add src/ui/panels/mod.rs src/ui/panels/top_bar.rs
git commit -m "refactor(ui): extract top bar to panels module"
```

### Task 2: Extract Bottom Timeline

**Files:**
- Create: `src/ui/panels/bottom_bar.rs`

- [ ] **Step 1: Create `src/ui/panels/bottom_bar.rs`**
```rust
use bevy::prelude::*;
use bevy_egui::egui;
use crate::game_state::EditorPhase;

pub fn show_bottom_bar(
    ctx: &egui::Context,
    current_phase: &EditorPhase,
    next_phase: &mut NextState<EditorPhase>,
    is_valid: bool,
) {
    egui::TopBottomPanel::bottom("phase_timeline").show(ctx, |ui| {
        ui.horizontal_centered(|ui| {
            let phases = [
                (EditorPhase::Shape, "1. Shape"),
                (EditorPhase::Factions, "2. Factions"),
                (EditorPhase::Landscape, "3. Landscape"),
                (EditorPhase::Sediments, "4. Sediments"),
                (EditorPhase::NPCs, "5. NPCs"),
                (EditorPhase::Plants, "6. Plants"),
                (EditorPhase::Height3D, "7. Height3D"),
            ];

            let current_idx = phases
                .iter()
                .position(|(p, _)| p == current_phase)
                .unwrap_or(0);

            for (idx, (phase, label)) in phases.into_iter().enumerate() {
                let is_current = current_phase == &phase;
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
}
```

- [ ] **Step 2: Commit Task 2**
```bash
git add src/ui/panels/bottom_bar.rs
git commit -m "refactor(ui): extract bottom bar to panels module"
```

### Task 3: Extract Tool Sidebar

**Files:**
- Create: `src/ui/panels/tools.rs`

- [ ] **Step 1: Create `src/ui/panels/tools.rs`**
```rust
use bevy::prelude::*;
use bevy_egui::egui;
use crate::game_state::{CurrentTool, EditorPhase, ShapeTool};
use crate::map::{TerrainType, ForestType};

pub fn show_tools(
    ctx: &egui::Context,
    current_phase: &EditorPhase,
    current_tool: &mut CurrentTool,
) {
    egui::SidePanel::left("tool_sidebar")
        .default_width(120.0)
        .show(ctx, |ui| {
            ui.heading("Tools");
            ui.separator();

            match current_phase {
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
                            ui.selectable_value(&mut current_tool.sediment, TerrainType::Dirt, "Dirt");
                            ui.selectable_value(&mut current_tool.sediment, TerrainType::Dusty, "Dusty");
                            ui.selectable_value(&mut current_tool.sediment, TerrainType::Fertile, "Fertile");
                            ui.selectable_value(&mut current_tool.sediment, TerrainType::Mossy, "Mossy");
                            ui.selectable_value(&mut current_tool.sediment, TerrainType::Steppe, "Steppe");
                            ui.selectable_value(&mut current_tool.sediment, TerrainType::Stony, "Stony");
                            ui.selectable_value(&mut current_tool.sediment, TerrainType::Swamp, "Swamp");
                        });

                    ui.separator();
                    ui.label("Forest Tool:");
                    ui.checkbox(&mut current_tool.active_forest_tool, "Active");
                    egui::ComboBox::from_id_salt("forest_type")
                        .selected_text(format!("{:?}", current_tool.forest_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut current_tool.forest_type, ForestType::None, "None");
                            ui.selectable_value(&mut current_tool.forest_type, ForestType::Deciduous, "Deciduous");
                            ui.selectable_value(&mut current_tool.forest_type, ForestType::Coniferous, "Coniferous");
                        });
                    ui.add(egui::Slider::new(&mut current_tool.forest_density, 0.0..=1.0).text("Density"));
                }
                EditorPhase::NPCs => {
                    ui.label("NPC Tools:");
                    let tools = [
                        (crate::game_state::NpcTool::None, "None"),
                        (crate::game_state::NpcTool::SpawnPoi, "Spawn POI"),
                        (crate::game_state::NpcTool::SpawnEnemyCamp, "Spawn Enemy Camp"),
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
                EditorPhase::Plants => {
                    ui.label("Bio-Deposit Tools:");
                    egui::ComboBox::from_id_salt("bio_resource_type")
                        .selected_text(format!("{:?}", current_tool.bio_resource))
                        .show_ui(ui, |ui| {
                            use crate::map::DepositType;
                            ui.selectable_value(&mut current_tool.bio_resource, DepositType::Rabbit, "Rabbit");
                            ui.selectable_value(&mut current_tool.bio_resource, DepositType::Deer, "Deer");
                            ui.selectable_value(&mut current_tool.bio_resource, DepositType::Boar, "Boar");
                            ui.selectable_value(&mut current_tool.bio_resource, DepositType::WildFlax, "WildFlax");
                            ui.selectable_value(&mut current_tool.bio_resource, DepositType::Raspberries, "Raspberries");
                            ui.selectable_value(&mut current_tool.bio_resource, DepositType::Pumpkin, "Pumpkin");
                            ui.selectable_value(&mut current_tool.bio_resource, DepositType::WildWheat, "WildWheat");
                            ui.selectable_value(&mut current_tool.bio_resource, DepositType::OceanFish, "OceanFish");
                        });
                    ui.add(egui::Slider::new(&mut current_tool.bio_amount, 1..=100).text("Amount"));
                    ui.add(egui::Slider::new(&mut current_tool.bio_brush_size, 1..=5).text("Brush Size"));
                }
                _ => {
                    ui.label("No tools for this phase.");
                }
            }
        });
}
```

- [ ] **Step 2: Commit Task 3**
```bash
git add src/ui/panels/tools.rs
git commit -m "refactor(ui): extract tool sidebar to panels module"
```

### Task 4: Extract Inspector Sidebar

**Files:**
- Create: `src/ui/panels/inspector.rs`

- [ ] **Step 1: Create `src/ui/panels/inspector.rs`**
```rust
use bevy::prelude::*;
use bevy_egui::egui;
use crate::game_state::{CurrentTool, EditorPhase, FactionManager, FactionType, Faction, NpcTool};
use crate::map::{MapData, PoiType};

pub fn show_inspector(
    ctx: &egui::Context,
    current_phase: &EditorPhase,
    current_tool: &mut CurrentTool,
    faction_manager: &mut FactionManager,
    map_data: &MapData,
) {
    let is_valid = map_data.validation_errors.is_empty();

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

                if *current_phase == EditorPhase::Factions || *current_phase == EditorPhase::NPCs {
                    ui.collapsing("🚩 Faction Hierarchy", |ui| {
                        let mut to_remove = None;
                        let mut to_select = None;
                        for (idx, faction) in faction_manager.factions.iter().enumerate() {
                            let is_selected = faction_manager.selected_faction == Some(faction.id);
                            ui.horizontal(|ui| {
                                if ui.selectable_label(is_selected, &faction.name).clicked() {
                                    to_select = Some(faction.id);
                                }
                                if faction.faction_type != FactionType::Player {
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
                            let next_id = faction_manager.factions.iter().map(|f| f.id).max().unwrap_or(0) + 1;
                            faction_manager.factions.push(Faction {
                                id: next_id,
                                name: format!("Faction {}", next_id),
                                faction_type: FactionType::Neutral,
                                color: Color::srgb(rand::random(), rand::random(), rand::random()),
                                economy_focus: "None".to_string(),
                            });
                        }
                    });
                }

                ui.collapsing("🔍 Selection Properties", |ui| {
                    if let Some(selected_id) = faction_manager.selected_faction {
                        if let Some(faction) = faction_manager.factions.iter_mut().find(|f| f.id == selected_id) {
                            ui.label(format!("Editing: {}", faction.name));
                            ui.text_edit_singleline(&mut faction.name);
                            ui.horizontal(|ui| {
                                ui.label("Type:");
                                egui::ComboBox::from_id_salt("faction_type_prop")
                                    .selected_text(format!("{:?}", faction.faction_type))
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut faction.faction_type, FactionType::Player, "Player");
                                        ui.selectable_value(&mut faction.faction_type, FactionType::Neutral, "Neutral");
                                        ui.selectable_value(&mut faction.faction_type, FactionType::Enemy, "Enemy");
                                    });
                            });
                            ui.horizontal(|ui| {
                                ui.label("Economy:");
                                egui::ComboBox::from_id_salt("economy_focus_prop")
                                    .selected_text(&faction.economy_focus)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut faction.economy_focus, "None".to_string(), "None");
                                        ui.selectable_value(&mut faction.economy_focus, "Mining".to_string(), "Mining");
                                        ui.selectable_value(&mut faction.economy_focus, "Farming".to_string(), "Farming");
                                        ui.selectable_value(&mut faction.economy_focus, "Woodcutting".to_string(), "Woodcutting");
                                    });
                            });
                        }
                    } else if current_tool.npc == NpcTool::SpawnEnemyCamp {
                        ui.label("Enemy Camp Settings:");
                        ui.add(egui::Slider::new(&mut current_tool.camp_difficulty, 0.0..=1.0).text("Difficulty"));
                        ui.add(egui::Slider::new(&mut current_tool.camp_power, 10..=1000).text("Combat Power"));
                    } else if current_tool.npc == NpcTool::SpawnPoi {
                        ui.label("POI Settings:");
                        egui::ComboBox::from_id_salt("poi_type_prop")
                            .selected_text(format!("{:?}", current_tool.poi_type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut current_tool.poi_type, PoiType::TradePost, "TradePost");
                                ui.selectable_value(&mut current_tool.poi_type, PoiType::Ruins, "Ruins");
                                ui.selectable_value(&mut current_tool.poi_type, PoiType::Shrine, "Shrine");
                                ui.selectable_value(&mut current_tool.poi_type, PoiType::Treasure, "Treasure");
                            });
                    } else {
                        ui.label("No selection.");
                    }
                });
            });
        });
}
```

- [ ] **Step 2: Commit Task 4**
```bash
git add src/ui/panels/inspector.rs
git commit -m "refactor(ui): extract inspector sidebar to panels module"
```

### Task 5: Refactor `src/ui/mod.rs` to Use Panels

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Update `src/ui/mod.rs` imports and dispatcher**
```rust
// ... existing imports ...
pub mod details;
pub mod logs;
pub mod resources;
pub mod panels; // Add this

// ... UiPlugin implementation ...

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
    let phase = *current_phase.get();

    panels::top_bar::show_top_bar(ctx, &phase, &mut terrain_config, &mut ev_rebuild);
    panels::bottom_bar::show_bottom_bar(ctx, &phase, &mut next_phase, is_valid);
    panels::tools::show_tools(ctx, &phase, &mut current_tool);
    panels::inspector::show_inspector(ctx, &phase, &mut current_tool, &mut faction_manager, &map_data);
}

// ... rest of setup_ui and tests ...
```

- [ ] **Step 2: Verify with `cargo check`**
Run: `cargo check --quiet`

- [ ] **Step 3: Run `cargo fmt`**
Run: `cargo fmt`

- [ ] **Step 4: Commit Task 5**
```bash
git add src/ui/mod.rs
git commit -m "refactor(ui): modularize editor UI and update dispatcher"
```
