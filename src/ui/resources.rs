use bevy::prelude::*;
use crate::economy::GlobalResources;

pub struct ResourceUiPlugin;

impl Plugin for ResourceUiPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Component)]
pub struct ResourceText;

pub fn setup_resource_ui(parent: &mut EntityCommands) {
    parent.with_children(|builder| {
        builder.spawn((
            Text::new("Food: 0"),
            TextFont { font_size: 20.0, ..default() },
            TextColor(Color::WHITE),
            ResourceText,
        ));
    });
}

pub fn update_resource_ui(
    resources: Res<GlobalResources>,
    mut query: Query<&mut Text, With<ResourceText>>,
) {
    if let Some(mut text) = query.iter_mut().next() {
        text.0 = format!("Resources: {:.0}", resources.food);
    }
}
