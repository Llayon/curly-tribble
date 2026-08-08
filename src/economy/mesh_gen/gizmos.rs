use crate::map::{
    CliffLowerSide, EdgeType, EnemyCamp, ForestType, MapData, PoiType, PointOfInterest, HEX_SIZE,
};
use bevy::prelude::*;

pub fn draw_cliffs_gizmos(
    mut gizmos: Gizmos,
    map_data: Res<MapData>,
    config: Res<crate::map::terrain_gen::TerrainConfig>,
) {
    if !config.cliff_layer.is_visible() {
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

            let draw_arrow = |g: &mut Gizmos, d: Vec3| {
                let arrow_base = midpoint + d * 0.15;
                let arrow_tip = midpoint + d * 0.35;
                g.line(
                    arrow_base + Vec3::Y * y,
                    arrow_tip + Vec3::Y * y,
                    Color::BLACK,
                );
                let head_left = arrow_tip - d * 0.1 + perp * 0.08;
                let head_right = arrow_tip - d * 0.1 - perp * 0.08;
                g.line(
                    arrow_tip + Vec3::Y * y,
                    head_left + Vec3::Y * y,
                    Color::BLACK,
                );
                g.line(
                    arrow_tip + Vec3::Y * y,
                    head_right + Vec3::Y * y,
                    Color::BLACK,
                );
            };

            match data.cliff_lower_side {
                CliffLowerSide::Unresolved => {
                    draw_arrow(&mut gizmos, dir);
                    draw_arrow(&mut gizmos, -dir);
                }
                CliffLowerSide::A => {
                    draw_arrow(&mut gizmos, -dir);
                }
                CliffLowerSide::B => {
                    draw_arrow(&mut gizmos, dir);
                }
            }
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

pub fn draw_hex_grid_gizmos(
    mut gizmos: Gizmos,
    map: Res<MapData>,
    topology: Res<crate::map::topology::TerrainTopology>,
    phase: Res<State<crate::game_state::EditorPhase>>,
) {
    let current_phase = *phase.get();
    if current_phase > crate::game_state::EditorPhase::Deposits {
        return;
    }

    let size = HEX_SIZE;
    let color = Color::srgba(0.0, 0.0, 0.0, 0.5);
    let sub_color = Color::srgba(0.25, 0.25, 0.25, 0.35);
    let show_subtriangles = matches!(
        current_phase,
        crate::game_state::EditorPhase::Balance
            | crate::game_state::EditorPhase::Height3D
            | crate::game_state::EditorPhase::Finetuning
            | crate::game_state::EditorPhase::Deposits
    );
    let is_3d = current_phase >= crate::game_state::EditorPhase::Height3D;
    let mode = if is_3d {
        crate::map::topology::TerrainHeightMode::Relief3D
    } else {
        crate::map::topology::TerrainHeightMode::Flat
    };
    let heights = crate::map::topology::compute_vertex_heights(&topology, &map, mode);

    if show_subtriangles && !topology.triangles.is_empty() {
        for tri in &topology.triangles {
            let idx0 = tri[0] as usize;
            let idx1 = tri[1] as usize;
            let idx2 = tri[2] as usize;
            if idx0 < topology.vertices_xz.len()
                && idx1 < topology.vertices_xz.len()
                && idx2 < topology.vertices_xz.len()
            {
                let p0 = topology.vertices_xz[idx0];
                let p1 = topology.vertices_xz[idx1];
                let p2 = topology.vertices_xz[idx2];
                let v0 = Vec3::new(p0.x, heights[idx0] + 0.02, p0.y);
                let v1 = Vec3::new(p1.x, heights[idx1] + 0.02, p1.y);
                let v2 = Vec3::new(p2.x, heights[idx2] + 0.02, p2.y);

                gizmos.line(v0, v1, sub_color);
                gizmos.line(v1, v2, sub_color);
                gizmos.line(v2, v0, sub_color);
            }
        }
    }

    for &coord in map.tiles.keys() {
        let center = coord.to_world(size);
        let mut points = Vec::with_capacity(6);
        for i in 0..6 {
            #[allow(clippy::cast_precision_loss)]
            let angle_deg = 60.0 * i as f32 + 30.0;
            let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
            let vx = center.x + size * angle_rad.cos();
            let vz = center.z + size * angle_rad.sin();
            let vy = if is_3d {
                let n_hex = crate::map::HexCoord::from_world(Vec3::new(vx, 0.0, vz), size);
                map.get_hex_height(n_hex.q, n_hex.r) + 0.02
            } else {
                0.02
            };
            points.push(Vec3::new(vx, vy, vz));
        }

        // Outer hex perimeter outline
        let mut perimeter = points.clone();
        perimeter.push(points[0]);
        gizmos.linestrip(perimeter, color);
        // 3. 1x1m Sub-cell grid placement nodes on Deposits phase (3D snapped)
        if current_phase == crate::game_state::EditorPhase::Deposits {
            let dot_color = Color::srgba(0.0, 0.8, 1.0, 0.6);
            let step = 1.0;
            let mut dx = -size;
            while dx <= size {
                let mut dz = -size;
                while dz <= size {
                    if dx * dx + dz * dz <= size * size {
                        let wx = center.x + dx;
                        let wz = center.z + dz;
                        let n_hex = crate::map::HexCoord::from_world(Vec3::new(wx, 0.0, wz), size);
                        let dot_y = if is_3d {
                            map.get_hex_height(n_hex.q, n_hex.r) + 0.02
                        } else {
                            0.02
                        };
                        let dot_pos = Vec3::new(wx, dot_y, wz);
                        gizmos.sphere(Isometry3d::from_translation(dot_pos), 0.08, dot_color);
                    }
                    dz += step;
                }
                dx += step;
            }
        }
    }
}

pub fn draw_forest_gizmos(
    mut gizmos: Gizmos,
    map_data: Res<MapData>,
    config: Res<crate::map::terrain_gen::TerrainConfig>,
) {
    if !config.forest_layer.is_visible() {
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
                ForestType::None => Color::NONE,
            };
            #[allow(clippy::cast_possible_truncation)]
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

pub fn draw_mines_gizmos(
    mut gizmos: Gizmos,
    map_data: Res<MapData>,
    q_mines: Query<&crate::map::mines::MineDeposit>,
) {
    let size = HEX_SIZE * 0.75;
    for mine in &q_mines {
        let coord = mine.hex_coord;
        let mut center = coord.to_world(HEX_SIZE);
        center.y = map_data.get_hex_height(coord.q, coord.r) + 0.05;

        let color = match mine.mine_type {
            crate::game_state::MineType::Coal => Color::srgb(0.15, 0.15, 0.15),
            crate::game_state::MineType::Iron => Color::srgb(0.75, 0.25, 0.1),
            crate::game_state::MineType::Copper => Color::srgb(0.85, 0.5, 0.15),
            crate::game_state::MineType::Gold => Color::srgb(0.95, 0.8, 0.1),
            crate::game_state::MineType::Stone => Color::srgb(0.55, 0.55, 0.6),
        };

        let mut points = Vec::new();
        for i in 0..6 {
            let angle_deg = 60.0 * i as f32 + 30.0;
            let angle_rad = std::f32::consts::PI / 180.0 * angle_deg;
            let vx = center.x + size * angle_rad.cos();
            let vz = center.z + size * angle_rad.sin();
            points.push(Vec3::new(vx, center.y, vz));
        }
        let first = points[0];
        points.push(first);
        for pair in points.windows(2) {
            gizmos.line(pair[0], pair[1], color);
        }

        let depth_len = match mine.depth {
            crate::game_state::MineDepth::Shallow => 1.0,
            crate::game_state::MineDepth::Medium => 2.5,
            crate::game_state::MineDepth::Deep => 4.5,
        };
        gizmos.line(center, center - Vec3::Y * depth_len, color);
    }
}
