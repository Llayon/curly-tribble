use crate::map::{
    EdgeDirection, EdgeType, EnemyCamp, ForestType, MapData, PoiType, PointOfInterest, HEX_SIZE,
};
use bevy::prelude::*;

pub fn draw_cliffs_gizmos(
    mut gizmos: Gizmos,
    map_data: Res<MapData>,
    config: Res<crate::map::terrain_gen::TerrainConfig>,
) {
    if !config.show_cliffs {
        return;
    }
    let size = HEX_SIZE;
    let y = 0.1;

    for (edge, data) in &map_data.edges {
        if data.edge_type == EdgeType::Cliff {
            let center_a = edge.a.to_world(size);
            let center_b = edge.b.to_world(size);
            let between = center_b - center_a;
            let dist = between.length();
            if dist < 0.001 {
                continue;
            }
            let dir = between / dist;
            let perp = Vec3::new(-dir.z, 0.0, dir.x);
            let midpoint = (center_a + center_b) * 0.5;
            let edge_half_len = size * 0.48;
            let start = midpoint - perp * edge_half_len;
            let end = midpoint + perp * edge_half_len;
            gizmos.line(start + Vec3::Y * y, end + Vec3::Y * y, Color::WHITE);
            let arrow_dir = if data.direction == EdgeDirection::Normal {
                dir
            } else {
                -dir
            };
            let arrow_base = midpoint + arrow_dir * 0.15;
            let arrow_tip = midpoint + arrow_dir * 0.35;
            gizmos.line(
                midpoint + Vec3::Y * y,
                arrow_tip + Vec3::Y * y,
                Color::WHITE,
            );
            let arrow_perp = Vec3::new(-arrow_dir.z, 0.0, arrow_dir.x) * 0.1;
            gizmos.line(
                arrow_tip + Vec3::Y * y,
                (arrow_base + arrow_perp) + Vec3::Y * y,
                Color::WHITE,
            );
            gizmos.line(
                arrow_tip + Vec3::Y * y,
                (arrow_base - arrow_perp) + Vec3::Y * y,
                Color::WHITE,
            );
        }
    }
}

pub fn draw_factions_gizmos(
    mut _gizmos: Gizmos,
    _map_data: Res<MapData>,
    _faction_manager: Res<crate::game_state::FactionManager>,
) {
}

pub struct GizmosPlugin;

impl Plugin for GizmosPlugin {
    fn build(&self, _app: &mut App) {}
}

pub fn draw_hex_grid_gizmos(mut gizmos: Gizmos, map: Res<MapData>) {
    let size = HEX_SIZE;
    let color = Color::BLACK;
    let y = 0.02;
    for &coord in map.tiles.keys() {
        let center = coord.to_world(size);
        let mut points = Vec::new();
        for i in 0..6 {
            let angle_deg = 60.0 * i as f32 + 30.0;
            let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
            let vx = center.x + size * angle_rad.cos();
            let vz = center.z + size * angle_rad.sin();
            points.push(Vec3::new(vx, y, vz));
        }
        let first = points[0];
        points.push(first);
        gizmos.linestrip(points, color);
    }
}

pub fn draw_forest_gizmos(
    mut gizmos: Gizmos,
    map_data: Res<MapData>,
    config: Res<crate::map::terrain_gen::TerrainConfig>,
) {
    if !config.show_forests {
        return;
    }
    let size = HEX_SIZE;
    let y = 0.1;
    for (coord, tile) in &map_data.tiles {
        if tile.forest_type != ForestType::None && tile.forest_density > 0.0 {
            let center = coord.to_world(size);
            let color = match tile.forest_type {
                ForestType::Deciduous => Color::srgb(0.0, 0.8, 0.2),
                ForestType::Coniferous => Color::srgb(0.0, 0.4, 0.1),
                _ => Color::NONE,
            };
            let density_count = (tile.forest_density * 5.0) as i32 + 1;
            for i in 0..density_count {
                let offset_x = (i as f32 * 1.3).cos() * 0.3;
                let offset_z = (i as f32 * 1.3).sin() * 0.3;
                let pos = center + Vec3::new(offset_x, 0.0, offset_z);
                gizmos.line(pos + Vec3::Y * y, pos + Vec3::Y * (y + 0.4), color);
            }
        }
    }
}

pub fn draw_npc_objects_gizmos(
    mut gizmos: Gizmos,
    q_pois: Query<&PointOfInterest>,
    q_camps: Query<&EnemyCamp>,
) {
    let size = HEX_SIZE;
    let y = 0.5;
    for poi in &q_pois {
        let center = poi.hex_coord.to_world(size) + Vec3::Y * y;
        let color = match poi.poi_type {
            PoiType::TradePost => Color::srgb(0.0, 1.0, 0.5),
            PoiType::Ruins => Color::srgb(0.6, 0.6, 0.6),
            PoiType::Shrine => Color::srgb(0.8, 0.0, 1.0),
            PoiType::Treasure => Color::srgb(1.0, 0.8, 0.0),
        };
        gizmos.sphere(center, 0.4, color);
    }
    for camp in &q_camps {
        let center = camp.hex_coord.to_world(size) + Vec3::Y * y;
        gizmos.sphere(center, 0.2, Color::srgb(1.0, 0.0, 0.0));
        gizmos.line(
            center + Vec3::Y * 0.6,
            center + Vec3::X * 0.4,
            Color::srgb(1.0, 0.0, 0.0),
        );
        gizmos.line(
            center + Vec3::Y * 0.6,
            center - Vec3::X * 0.4,
            Color::srgb(1.0, 0.0, 0.0),
        );
        gizmos.line(
            center + Vec3::Y * 0.6,
            center + Vec3::Z * 0.4,
            Color::srgb(1.0, 0.0, 0.0),
        );
        gizmos.line(
            center + Vec3::Y * 0.6,
            center - Vec3::Z * 0.4,
            Color::srgb(1.0, 0.0, 0.0),
        );
    }
}
