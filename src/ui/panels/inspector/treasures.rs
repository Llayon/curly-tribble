use crate::map::{ArtifactType, ResourceType, TreasureDeposit, TreasureItem};
use bevy::prelude::*;
use bevy_egui::egui;

pub struct TreasureInspectorPlugin;

impl Plugin for TreasureInspectorPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn show_treasure_properties(ui: &mut egui::Ui, deposit: &mut TreasureDeposit) {
    ui.collapsing("💰 Treasure Contents", |ui| {
        let mut to_remove = None;
        for (idx, item) in deposit.contents.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{:?}", item));
                if ui.button("🗑").clicked() {
                    to_remove = Some(idx);
                }
            });
        }
        if let Some(idx) = to_remove {
            deposit.contents.remove(idx);
        }

        ui.separator();
        ui.label("Add Item:");
        ui.horizontal(|ui| {
            if ui.button("+ Gold").clicked() {
                deposit.contents.push(TreasureItem::Gold(100));
            }
            if ui.button("+ Wood").clicked() {
                deposit.contents.push(TreasureItem::Resources {
                    resource: ResourceType::Wood,
                    amount: 50,
                });
            }
            if ui.button("+ Relic").clicked() {
                deposit
                    .contents
                    .push(TreasureItem::ArtifactDef(ArtifactType::AncientRelic));
            }
        });
    });
}
