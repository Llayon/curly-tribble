# Phase 6: Bio-Deposits Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement harvestable biological resources (fauna, flora, and fish) as individual ECS entities with billboard visualization, habitat validation, and professional editor tools.

**Architecture:** 
- Use a central `ResourceDeposit` component for data storage.
- Implement billboard-based rendering for 2D icons in a 3D world.
- Add a hybrid "Select Entity" UI for precise and brush-based placement.
- Implement cached validation logic to provide real-time habitat feedback (e.g., "Forest required for Boars").

**Tech Stack:** Bevy 0.18.1, egui, rand, noise.

---

### Task 1: Data Model & ECS Setup

**Files:**
- Modify: `src/map/zoning.rs` (Refactored to `src/map/deposits.rs`)

- [x] **Step 1: Define `DepositType` and `ResourceDeposit`**
- [x] **Step 2: Register types in `ZoningPlugin`**
- [x] **Step 3: Run `cargo check --quiet` to verify types**

---

### Task 2: Tool State & UI Integration

**Files:**
- Modify: `src/game_state.rs`
- Modify: `src/ui/mod.rs` (Modularized to `src/ui/panels/`)

- [x] **Step 1: Add Bio Tools to `CurrentTool`**
- [x] **Step 2: Implement Phase 6 UI Sidebar**
- [x] **Step 3: Add "Plants" to the bottom timeline**

---

### Task 3: Interactive Placement System (Click & Brush)

**Files:**
- Modify: `src/map/tools/bio.rs`

- [x] **Step 1: Implement `handle_bio_tools`**
- [x] **Step 2: Register system in `src/map/mod.rs` Update schedule**

---

### Task 4: Billboard Visualization

**Files:**
- Create: `src/economy/mesh_gen/billboards.rs`
- Modify: `src/economy/mesh_gen/mod.rs`

- [x] **Step 1: Create `draw_bio_billboards` gizmo system**

---

### Task 5: Habitat Validation & Auto-Fill

**Files:**
- Modify: `src/map/generation/npcs.rs`
- Modify: `src/map/validation.rs`

- [x] **Step 1: Implement `auto_spawn_bio_deposits`**
- [x] **Step 2: Implement continuous habitat validator**
