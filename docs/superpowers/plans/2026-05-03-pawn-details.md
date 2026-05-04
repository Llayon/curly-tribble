# The Dark Narrative: Phase 3 - Pawn Details UI Plan

**Goal:** Show a detail panel when a pawn is selected, displaying its Name and Hunger status.

**Architecture:**
- Use Bevy's built-in `Name` component for pawns.
- Add a `SelectionPanel` component to a UI node.
- Update UI text systems to query for the `Selected` entity.

---

### Task 1: Add Name and Identity to Pawns

**Files:**
- Modify: `src/pawn/mod.rs`

- [ ] **Step 1: Add Name to test pawn**
Include `Name` in the spawn bundle.

```rust
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::from_size(Vec3::splat(0.5)))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        Transform::from_xyz(0.0, 0.25, 0.0),
        Pawn,
        Hunger(0.0),
        Name::new("Erik the Red"), // New component
    ))
```

- [ ] **Step 2: Commit**
```bash
git add src/pawn/mod.rs
git commit -m "feat: add Name component to test pawn"
```

---

### Task 2: Implement Detail UI Panel

**Files:**
- Modify: `src/ui.rs`

- [ ] **Step 1: Define Detail UI components**
Add `PawnDetailText` and `PawnDetailPanel`.

- [ ] **Step 2: Update setup_ui to include the panel**
Add a new node at the bottom-right for pawn details.

- [ ] **Step 3: Implement update_pawn_detail_ui system**
This system queries for `(With<Pawn>, With<Selected>, With<Name>, With<Hunger>)`.

```rust
fn update_pawn_detail_ui(
    selected_pawn: Query<(&Name, &Hunger), With<Selected>>,
    mut ui_query: Query<&mut Text, With<PawnDetailText>>,
) {
    if let Ok((name, hunger)) = selected_pawn.get_single() {
        for mut text in &mut ui_query {
            text.0 = format!("Name: {}\nHunger: {:.1}%", name, hunger.0);
        }
    } else {
        for mut text in &mut ui_query {
            text.0 = "No Selection".to_string();
        }
    }
}
```

- [ ] **Step 4: Commit**
```bash
git add src/ui.rs
git commit -m "feat: implement pawn detail UI panel"
```

---

### Task 3: Deselection Logic

**Files:**
- Modify: `src/main.rs` or `src/camera.rs`

- [ ] **Step 1: Allow clicking ground to deselect**
Add an observer to the ground tiles to remove `Selected` from all entities.

- [ ] **Step 2: Commit**
```bash
git add .
git commit -m "feat: add ground click deselection"
```
