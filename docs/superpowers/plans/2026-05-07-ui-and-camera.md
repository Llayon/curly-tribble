# Advanced Interaction (Camera & UI) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix UI visibility with a 2D camera, add an on-screen game log, and implement a smooth orbit camera with WASD/QE/Zoom controls.

**Architecture:** 
- Add `Camera2d` for UI rendering.
- Create `src/ui/logs.rs` to display `GameLogMessage` events.
- Refactor `src/camera.rs` to use a `CameraFocus` point and orbit-based movement calculations.

**Tech Stack:** Bevy 0.18.

---

### Task 1: UI Visibility Fix (2D Camera)

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Add Camera2d to setup_ui**
In Bevy 0.18.1, UI often requires an explicit 2D camera to render correctly over 3D scenes.

```rust
// src/ui/mod.rs

fn setup_ui(mut commands: Commands) {
    // 0. Explicit 2D Camera for UI
    commands.spawn(Camera2d::default());

    // 1. Top-left: Global Resources
    // ... rest of code
}
```

- [ ] **Step 2: Verify UI visibility**
Run the game and check if the "Resources: 0" text and "NO SURVIVOR SELECTED" panel are visible.

- [ ] **Step 3: Commit**
```bash
git add src/ui/mod.rs
git commit -m "fix(ui): add explicit Camera2d to ensure UI visibility"
```

---

### Task 2: On-Screen Game Log

**Files:**
- Create: `src/ui/logs.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: Create Game Log module**
Define the log widget structure.

```rust
// src/ui/logs.rs
use bevy::prelude::*;
use crate::events::{GameLogMessage, LogSeverity};

#[derive(Component)]
pub struct GameLogText;

pub struct GameLogPlugin;

impl Plugin for GameLogPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update_game_log);
    }
}

pub fn setup_log_ui(parent: &mut ChildBuilder) {
    parent.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            left: Val::Px(10.0),
            padding: UiRect::all(Val::Px(10.0)),
            max_width: Val::Px(400.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
    )).with_children(|log_node| {
        log_node.spawn((
            Text::new(""),
            TextFont { font_size: 16.0, ..default() },
            GameLogText,
        ));
    });
}

// Global buffer to persist logs between frames (simplified for now)
#[derive(Resource, Default)]
struct LogBuffer(Vec<(String, LogSeverity)>);

fn update_game_log(
    mut messages: MessageReader<GameLogMessage>,
    mut query: Query<&mut Text, With<GameLogText>>,
    mut buffer: Local<Vec<(String, LogSeverity)>>,
) {
    let mut changed = false;
    for msg in messages.read() {
        buffer.push((msg.message.clone(), msg.severity));
        if buffer.len() > 5 { buffer.remove(0); }
        changed = true;
    }

    if changed {
        if let Some(mut text) = query.iter_mut().next() {
            text.0 = String::new();
            // Build the string with colors if possible, but 0.18.1 Text is simple.
            // For now, just multi-line text.
            for (msg, _sev) in buffer.iter() {
                text.0.push_str(&format!("{}\n", msg));
            }
        }
    }
}
```

- [ ] **Step 2: Integrate Log UI**
Register the plugin and call setup in `src/ui/mod.rs`.

```rust
// src/ui/mod.rs
pub mod logs;

// In UiPlugin::build:
app.add_plugins((resources::ResourceUiPlugin, details::DetailUiPlugin, logs::GameLogPlugin));

// In setup_ui:
commands.spawn(Node {
    position_type: PositionType::Absolute,
    bottom: Val::Px(10.0),
    left: Val::Px(10.0),
    ..default()
}).with_children(|parent| {
    logs::setup_log_ui(parent);
});
```

- [ ] **Step 3: Test log display**
Click on a survivor to trigger a selection message and verify it appears in the bottom-left.

- [ ] **Step 4: Commit**
```bash
git add src/ui/
git commit -m "feat(ui): implement on-screen game log"
```

---

### Task 3: Orbit Camera Infrastructure

**Files:**
- Modify: `src/camera.rs`

- [ ] **Step 1: Define CameraFocus component**
This will be the anchor point for the camera.

```rust
// src/camera.rs

#[derive(Component)]
pub struct CameraFocus(pub Vec3);

#[derive(Component)]
pub struct CameraConfig {
    pub distance: f32,
    pub azimuth: f32, // Rotation around Y
    pub pitch: f32,   // Tilt angle
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            distance: 15.0,
            azimuth: 0.0,
            pitch: 0.8, // Radian (~45 deg)
        }
    }
}
```

- [ ] **Step 2: Update setup_camera**
Initialize with the new components.

```rust
fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 15.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
        CameraFocus(Vec3::ZERO),
        CameraConfig::default(),
    ));
}
```

- [ ] **Step 3: Commit**
```bash
git add src/camera.rs
git commit -m "refactor(camera): add Focus and Config components for orbit logic"
```

---

### Task 4: Advanced Camera Controls (Orbit Logic)

**Files:**
- Modify: `src/camera.rs`

- [ ] **Step 1: Rewrite move_camera**
Implement WASD relative movement, Q/E rotation, and Wheel zoom.

```rust
fn move_camera(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel: EventReader<bevy::input::mouse::MouseWheel>,
    mut query: Query<(&mut Transform, &mut CameraFocus, &mut CameraConfig), With<Camera>>,
    time: Res<Time>,
) {
    let Ok((mut transform, mut focus, mut config)) = query.get_single_mut() else { return };

    // 1. Rotation (Q/E)
    let rot_speed = 2.0;
    if keyboard.pressed(KeyCode::KeyQ) { config.azimuth += rot_speed * time.delta_secs(); }
    if keyboard.pressed(KeyCode::KeyE) { config.azimuth -= rot_speed * time.delta_secs(); }

    // 2. Zoom (Mouse Wheel)
    for event in mouse_wheel.read() {
        config.distance -= event.y * 2.0;
        config.distance = config.distance.clamp(5.0, 40.0);
        // Adjust pitch based on zoom (higher zoom = steeper angle)
        config.pitch = (config.distance / 40.0).clamp(0.5, 1.2);
    }

    // 3. Movement (WASD) - Relative to camera rotation
    let move_speed = 15.0;
    let mut move_dir = Vec3::ZERO;
    
    // Calculate forward/right based on azimuth
    let forward = Vec3::new(config.azimuth.sin(), 0.0, config.azimuth.cos()).normalize_or_zero();
    let right = Vec3::new(forward.z, 0.0, -forward.x); // Perpendicular

    if keyboard.pressed(KeyCode::KeyW) { move_dir -= forward; }
    if keyboard.pressed(KeyCode::KeyS) { move_dir += forward; }
    if keyboard.pressed(KeyCode::KeyA) { move_dir -= right; }
    if keyboard.pressed(KeyCode::KeyD) { move_dir += right; }

    focus.0 += move_dir.normalize_or_zero() * move_speed * time.delta_secs();

    // 4. Update Transform (Orbit math)
    let x = config.distance * config.azimuth.sin() * config.pitch.cos();
    let y = config.distance * config.pitch.sin();
    let z = config.distance * config.azimuth.cos() * config.pitch.cos();
    
    transform.translation = focus.0 + Vec3::new(x, y, z);
    transform.looking_at(focus.0, Vec3::Y);
}
```

- [ ] **Step 2: Update camera unit tests**
Tests need to account for the new components.

- [ ] **Step 3: Final Verification**
Test all controls in game. Ensure WASD feels natural regardless of Q/E rotation.

- [ ] **Step 4: Commit**
```bash
git add src/camera.rs
git commit -m "feat(camera): implement full orbit controls (Move, Rotate, Zoom)"
```
