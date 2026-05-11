# Fix Stuck Settler & Morale Drain Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve the bug where settlers get stuck at map corners, starve, and lose morale.

**Architecture:** 
1. Improve goal selection by picking the closest resource instead of the first one.
2. Robustify interaction logic by using 2D distance checks to account for verticality.
3. Fix morale drain by correctly identifying lanterns as light sources.
4. Correct map height calculations.

**Tech Stack:** Bevy 0.18.1, Rust.

---

### Task 1: Reproduction Test Case

**Files:**
- Create: `tests/stuck_settler_repro.rs`

- [ ] **Step 1: Write the failing test**
Create a test that spawns a settler and a bush with a height difference and verifies hunger doesn't decrease even when the settler is at the bush's horizontal position.

- [ ] **Step 2: Run test to verify it fails**
Run: `cargo test --test stuck_settler_repro`
Expected: FAIL (settler hunger increases instead of decreasing).

- [ ] **Step 3: Commit**
```bash
git add tests/stuck_settler_repro.rs
git commit -m "test: add reproduction case for stuck settler bug"
```

### Task 2: Fix Interaction Distance & Goal Selection

**Files:**
- Modify: `src/pawn/brain.rs`

- [ ] **Step 1: Implement closest bush selection in `find_resources`**
Update the query and logic to find the bush closest to the settler.

- [ ] **Step 2: Use 2D distance for interaction check in `collect_berries`**
Change `distance()` to a 2D distance check (ignoring Y) to handle verticality issues.

- [ ] **Step 3: Run reproduction test to verify fix**
Run: `cargo test --test stuck_settler_repro`
Expected: PASS.

- [ ] **Step 4: Commit**
```bash
git add src/pawn/brain.rs
git commit -m "fix: closest bush selection and 2D distance interaction"
```

### Task 3: Fix Morale & Lantern Light

**Files:**
- Modify: `src/pawn/mod.rs`

- [ ] **Step 1: Add `LightSource` to Pioneer Lantern**
Ensure the lantern spawned for pioneers includes the `LightSource` component so `detect_darkness` recognizes it.

- [ ] **Step 2: Commit**
```bash
git add src/pawn/mod.rs
git commit -m "fix: add LightSource to pioneer lanterns"
```

### Task 4: Fix Map Height Calculation

**Files:**
- Modify: `src/map/zoning.rs`

- [ ] **Step 1: Fix `get_corner_height` loop range**
Change `-1..0` to `-1..=0` to correctly include both tiles on the axis.

- [ ] **Step 2: Commit**
```bash
git add src/map/zoning.rs
git commit -m "fix: correct get_corner_height loop range"
```

### Task 5: Final Verification

- [ ] **Step 1: Run all tests**
Run: `cargo test`
Expected: ALL PASS.

- [ ] **Step 2: Run clippy**
Run: `cargo clippy`
Expected: ZERO WARNINGS.
