use bevy::prelude::*;
use crate::economy::GlobalResources;
use crate::events::{GameLogMessage, LogSeverity};
use crate::pawn::{Hunger, Sanity, Settler, Selected};
use crate::sets::GameSet;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_ui)
           .add_systems(Update, (
               update_resource_ui, 
               handle_game_logs,
               update_settler_detail_ui
           ).in_set(GameSet::Visuals));
    }
}

#[derive(Component)]
struct ResourceText;

#[derive(Component)]
struct SettlerDetailText;

fn handle_game_logs(mut messages: MessageReader<GameLogMessage>) {
    for message in messages.read() {
        match message.severity {
            LogSeverity::Info => info!("[LOG] {}", message.message),
            LogSeverity::Warning => warn!("[WARN] {}", message.message),
            LogSeverity::DarkEvent => error!("[DARK] {}", message.message),
        }
    }
}

fn setup_ui(mut commands: Commands) {
    // 1. Top-left: Global Resources
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
    )).with_children(|parent| {
        parent.spawn((
            Text::new("Food: 0"),
            TextFont { font_size: 20.0, ..default() },
            TextColor(Color::WHITE),
            ResourceText,
        ));
    });

    // 2. Bottom-right: Settler Details
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(15.0)),
            min_width: Val::Px(250.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.05, 0.0, 0.9)), // Darker, brownish tint
    )).with_children(|parent| {
        parent.spawn((
            Text::new("No Survivor Selected"),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::srgb(0.9, 0.8, 0.7)),
            SettlerDetailText,
        ));
    });
}

fn update_resource_ui(
    resources: Res<GlobalResources>,
    mut query: Query<&mut Text, With<ResourceText>>,
) {
    if let Some(mut text) = query.iter_mut().next() {
        text.0 = format!("Food Rations: {:.0}", resources.food);
    }
}

fn update_settler_detail_ui(
    selected_settler: Query<(&Name, &Hunger, &Sanity), (With<Settler>, With<Selected>)>,
    mut ui_query: Query<&mut Text, With<SettlerDetailText>>,
) {
    // Используем безопасный паттерн из нашего архитектурного кодекса
    let mut text = if let Some(t) = ui_query.iter_mut().next() { t } else { return; };
    
    if let Some((name, hunger, sanity)) = selected_settler.iter().next() {
        text.0 = format!(
            "IDENT: {}\n\nHUNGER: {:.1}%\nSANITY: {:.1}%", 
            name.to_uppercase(), 
            hunger.0, 
            sanity.0
        );
    } else {
        text.0 = "NO SURVIVOR SELECTED".to_string();
    }
}
