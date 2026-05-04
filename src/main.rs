mod camera;
mod map;
mod pawn;
mod economy;
mod ui;
mod game_state;
mod events;
mod sets;

use camera::CameraPlugin;
use map::MapPlugin;
use pawn::PawnPlugin;
use economy::EconomyPlugin;
use ui::UiPlugin;
use game_state::GameStatePlugin;
use events::EventsPlugin;
use sets::SetsPlugin;
use bevy_ai_remote::BevyAiRemotePlugin;
use bevy::prelude::*;

// --- CONSTANTS ---
const WINDOW_TITLE: &str = "Ant Farm: Dark Narrative";
const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

fn main() {
    // 4. Panic Hook - Better error reporting for Windows/CLI
    std::panic::set_hook(Box::new(|info| {
        error!("Panic occurred: {:?}", info);
    }));

    App::new()
        // 1. Plugins Configuration via .set()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: WINDOW_TITLE.into(),
                resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                present_mode: bevy::window::PresentMode::AutoVsync,
                ..default()
            }),
            ..default()
        }).set(bevy::log::LogPlugin {
            filter: "info,wgpu_core=warn,wgpu_hal=warn,bevy_ai_remote=debug".into(),
            level: bevy::log::Level::INFO,
            ..default()
        }).set(AssetPlugin {
            // Default to linear filtering, but easily swappable
            ..default()
        }))
        // 2. Grouped Registration
        // External/Integration Plugins
        .add_plugins((
            BevyAiRemotePlugin,
            MeshPickingPlugin,
        ))
        // Internal Game Plugins
        .add_plugins((
            SetsPlugin,
            EventsPlugin,
            GameStatePlugin,
            EconomyPlugin,
            CameraPlugin,
            MapPlugin,
            PawnPlugin,
            UiPlugin,
        ))
        // 3. Conditional Debug Tools
        .add_systems(Update, |_keyboard: Res<ButtonInput<KeyCode>>| {
            #[cfg(debug_assertions)]
            {
                // Space for debug-only systems, like toggling inspector
            }
        })
        .run();
}
