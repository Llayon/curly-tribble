# Design Spec: Advanced Interaction (Camera & UI)

**Status:** Implemented
**Date:** 2026-05-07
**Topic:** UI Visibility and Enhanced Camera Controls

## 1. Problem Statement
- **UI Invisibility:** The current UI is often not rendered correctly because it lacks an explicit 2D camera.
- **Limited Navigation:** Current WASD controls are static and don't allow for rotation or zoom, making it hard to inspect the colony.
- **Invisible Feedback:** Game logs are only visible in the console, depriving the player of immediate narrative feedback.

## 2. Proposed Solution (Option A: Orbit-Style Controls)

### 2.1. Orbit Camera System
Instead of moving the camera directly, we move a "Focus Point" on the ground.
- **Focus Point:** A virtual point that WASD moves.
- **Orbit Logic:**
  - **Move (WASD):** Moves the focus point relative to the camera's forward/right vectors.
  - **Rotate (Q/E):** Rotates the camera's offset around the Y-axis of the focus point.
  - **Zoom (Wheel):** Adjusts the distance from the focus point and the pitch angle (0.1 to 2.0 scale).

### 2.2. UI Overlay Fix
- **Camera2d:** Spawn a dedicated `Camera2d` entity to handle the UI layer.
- **Z-Order:** Ensure UI nodes are rendered on top of the 3D scene.

### 2.3. On-Screen Game Log
- **Log Widget:** A transparent container in the bottom-left.
- **Reactive Messages:** A system that subscribes to `GameLogMessage` and updates the UI text.
- **Buffer:** Display the last 5 messages with severity-based coloring (White for Info, Yellow for Warning, Red for Dark Events).

## 3. Technical Specs
- **New Component:** `CameraFocus(Vec3)` to track the target.
- **Modified Plugin:** `CameraPlugin` in `src/camera.rs`.
- **Modified Plugin:** `UiPlugin` in `src/ui/mod.rs`.
- **New Module:** `src/ui/logs.rs` for the log widget logic.

## 4. Success Criteria
- [x] UI (Resources and Details) is visible immediately on startup.
- [x] WASD moves the camera relative to its rotation.
- [x] Q/E rotates the view smoothly.
- [x] Mouse wheel zooms in/out with pitch adjustment.
- [x] Console logs also appear in the bottom-left corner of the screen.
