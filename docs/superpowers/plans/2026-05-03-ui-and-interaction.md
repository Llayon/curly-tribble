# The Dark Narrative: UI & Interaction Implementation Plan

**Goal:** Fix visual glitches, add a resource display UI, and implement pawn selection.

**Architecture:**
- Use `bevy_ui` for a persistent resource bar.
- Use `bevy_picking` for entity selection.
- Refactor map visuals to eliminate Z-fighting.

**Tech Stack:** Bevy 0.18.

---

### Task 1: Fix Visual Z-Fighting

**Files:**
- Modify: `src/main.rs`
- Modify: `src/map/mod.rs`

- [ ] **Step 1: Remove redundant ground plane**
In `src/main.rs`, remove the green plane spawned in `setup`.

- [ ] **Step 2: Update tile colors**
In `src/map/mod.rs`, change the tile material to a more "ground-like" green/brown.

- [ ] **Step 3: Commit**
```bash
git add src/main.rs src/map/mod.rs
git commit -m "fix: eliminate Z-fighting by removing redundant ground plane"
```

---

### Task 2: Resource UI Bar

**Files:**
- Create: `src/ui.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Create UI Plugin**
In `src/ui.rs`, implement a system that spawns a text node at the top of the screen.

```rust
use bevy::prelude::*;
use crate::economy::GlobalResources;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
           .add_systems(Update, update_resource_ui);
    }
}

#[derive(Component)]
struct ResourceText;

fn setup_ui(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Food: 0"),
            TextFont {
                font_size: 20.0,
                ..default()
            },
            TextColor(Color::WHITE),
            ResourceText,
        ));
    });
}

fn update_resource_ui(
    resources: Res<GlobalResources>,
    mut query: Query<&mut Text, With<ResourceText>>,
) {
    for mut text in &mut query {
        text.0 = format!("Food: {:.0}", resources.food);
    }
}
```

- [ ] **Step 2: Register UiPlugin in main.rs**

- [ ] **Step 3: Commit**
```bash
git add src/ui.rs src/main.rs
git commit -m "feat: add resource UI bar"
```

---

### Task 3: Pawn Selection

**Files:**
- Modify: `src/main.rs`
- Modify: `src/pawn/mod.rs`

- [ ] **Step 1: Enable Picking Plugins**
In `main.rs`, add `MeshPickingPlugin`.

- [ ] **Step 2: Add Selection visual**
In `src/pawn/mod.rs`, add a component to track selection and a system to change the pawn's color when selected.

```rust
#[derive(Component, Default)]
pub struct Selected;

pub fn handle_selection(
    mut commands: Commands,
    query: Query<(Entity, &Pointer<Click>), With<Pawn>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    selected_query: Query<Entity, With<Selected>>,
) {
    // Basic logic: click to select, shift-click for multiple? 
    // For now: click to select one, deselect others.
}
```

Wait, simplified version for 0.18:
We can use `On::<Pointer<Down>>::run(...)`.

- [ ] **Step 3: Commit**
```bash
git add .
git commit -m "feat: implement basic pawn selection"
```
