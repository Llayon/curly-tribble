use bevy::prelude::*;
use crate::pawn::{Hunger, Morale, Settler, Selected};

#[derive(Component)]
pub struct SettlerDetailText;

pub fn setup_detail_ui(parent: &mut EntityCommands) {
    parent.with_children(|builder| {
        builder.spawn((
            Text::new("NO SURVIVOR SELECTED"),
            TextFont { font_size: 18.0, ..default() },
            TextColor(Color::srgb(0.8, 0.8, 1.0)),
            SettlerDetailText,
        ));
    });
}

pub fn update_settler_detail_ui(
    selected_settler: Query<(&Name, &Hunger, &Morale), (With<Settler>, With<Selected>, Or<(Changed<Hunger>, Changed<Morale>, Added<Selected>)>)>,
    mut ui_query: Query<&mut Text, With<SettlerDetailText>>,
    mut removed_selected: RemovedComponents<Selected>,
) {
    let mut text = if let Some(t) = ui_query.iter_mut().next() { t } else { return; };
    
    if let Some((name, hunger, morale)) = selected_settler.iter().next() {
        text.0 = format!(
            "PIONEER: {}\n\nHUNGER: {:.1}%\nMORALE: {:.1}%", 
            name.to_uppercase(), 
            hunger.value(), 
            morale.value()
        );
    } else if removed_selected.read().next().is_some() {
        text.0 = "NO SURVIVOR SELECTED".to_string();
    }
}

pub struct DetailUiPlugin;
impl Plugin for DetailUiPlugin {
    fn build(&self, _app: &mut App) {}
}
