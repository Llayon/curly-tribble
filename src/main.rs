use bevy::prelude::*;
use bevy_ai_remote::BevyAiRemotePlugin;
use savage_fantasy::{camera, economy, events, game_state, map, pawn, sets, ui};

// --- CONSTANTS ---
const WINDOW_TITLE: &str = "Savage Fantasy";
const WINDOW_WIDTH: u32 = 1280;
const WINDOW_HEIGHT: u32 = 720;

fn main() {
    // 4. Panic Hook - Better error reporting for Windows/CLI
    std::panic::set_hook(Box::new(|info| {
        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        };
        let _ = std::fs::write("panic_log.txt", format!("Panic occurred: {}\nLocation: {:?}", msg, info.location()));
        error!("Panic occurred: {:?}", info);
    }));

    App::new()
        // 1. Plugins Configuration via .set()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: WINDOW_TITLE.into(),
                        resolution: (WINDOW_WIDTH, WINDOW_HEIGHT).into(),
                        present_mode: bevy::window::PresentMode::AutoVsync,
                        ..default()
                    }),
                    ..default()
                })
                .set(bevy::log::LogPlugin {
                    filter: "info,wgpu_core=warn,wgpu_hal=warn,bevy_ai_remote=debug".into(),
                    level: bevy::log::Level::INFO,
                    ..default()
                })
                .set(AssetPlugin { ..default() }),
        )
        // 2. Grouped Registration
        // External/Integration Plugins
        .add_plugins((
            bevy_egui::EguiPlugin::default(),
            BevyAiRemotePlugin,
            MeshPickingPlugin,
        ))
        // Internal Game Plugins
        .add_plugins((
            sets::SetsPlugin,
            events::EventsPlugin,
            game_state::GameStatePlugin,
            economy::EconomyPlugin,
            camera::CameraPlugin,
            map::MapPlugin,
            map::atmosphere::AtmospherePlugin,
            pawn::PawnPlugin,
        ))
        // 3. UI and Finalization
        .add_plugins(ui::UiPlugin)
        // 4. Conditional Debug Tools
        .add_systems(Update, |_keyboard: Res<ButtonInput<KeyCode>>| {
            #[cfg(debug_assertions)]
            {
                // Space for debug-only systems, like toggling inspector
            }
        })
        .run();
}
