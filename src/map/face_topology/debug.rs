//! Optional immediate-mode diagnostics for `HexFaceTopology`.
use crate::game_state::EditorPhase;
use crate::map::face_topology::corner_key::{canonical_corner_key, regular_corner_position};
use crate::map::face_topology::runtime::regenerate_hex_face_topology;
use crate::map::face_topology::types::{HexFaceTopology, VertexId};
use crate::map::MapData;
use crate::sets::GameSet;
use bevy::prelude::*;
use std::collections::HashSet;

const REGULAR_Y_OFFSET: f32 = 0.025;
const WARPED_Y_OFFSET: f32 = 0.035;
const VERTEX_Y_OFFSET: f32 = 0.045;
const ARROW_Y_OFFSET: f32 = 0.055;
const HALF_EDGE_DRAW_STRIDE: usize = 16;

#[derive(Resource, Debug, Clone, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub struct HexFaceDebugSettings {
    pub enabled: bool,
    pub show_regular_outlines: bool,
    pub show_warped_outlines: bool,
    pub show_shared_vertices: bool,
    pub show_half_edge_directions: bool,
}

impl Default for HexFaceDebugSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            show_regular_outlines: true,
            show_warped_outlines: true,
            show_shared_vertices: false,
            show_half_edge_directions: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UniqueUndirectedEdge {
    pub min: VertexId,
    pub max: VertexId,
}

#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct HexFaceDebugCache {
    pub edges: Vec<UniqueUndirectedEdge>,
    pub shared_vertices: Vec<VertexId>,
}

impl HexFaceDebugCache {
    pub fn rebuild(&mut self, topology: &HexFaceTopology) {
        self.edges = extract_unique_undirected_edges(topology);
        self.shared_vertices = extract_shared_vertices(topology);
    }

    pub fn clear(&mut self) {
        self.edges.clear();
        self.shared_vertices.clear();
    }
}

/// Extracts each topology edge once using `VertexId` identity, never positions.
#[must_use]
pub fn extract_unique_undirected_edges(topology: &HexFaceTopology) -> Vec<UniqueUndirectedEdge> {
    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    for edge in &topology.half_edges {
        let (min, max) = if edge.origin <= edge.destination {
            (edge.origin, edge.destination)
        } else {
            (edge.destination, edge.origin)
        };
        let unique_edge = UniqueUndirectedEdge { min, max };
        if seen.insert(unique_edge) {
            edges.push(unique_edge);
        }
    }
    edges
}

/// Returns one marker identity per canonical stored `MapVertex`.
#[must_use]
pub fn extract_shared_vertices(topology: &HexFaceTopology) -> Vec<VertexId> {
    (0..topology.vertices.len()).map(VertexId::new).collect()
}

#[must_use]
pub fn debug_overlay_visible(settings: &HexFaceDebugSettings, phase: EditorPhase) -> bool {
    settings.enabled && phase <= EditorPhase::Balance
}

pub struct FaceTopologyDebugPlugin;

impl Plugin for FaceTopologyDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HexFaceDebugSettings>()
            .init_resource::<HexFaceDebugCache>()
            .add_systems(Update, toggle_debug_settings.in_set(GameSet::Input))
            .add_systems(
                Update,
                draw_face_topology_debug
                    .after(regenerate_hex_face_topology)
                    .in_set(GameSet::Visuals),
            );
    }
}

fn toggle_debug_settings(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<HexFaceDebugSettings>,
) {
    if keyboard.just_pressed(KeyCode::F7) {
        settings.enabled = !settings.enabled;
    }
    if keyboard.just_pressed(KeyCode::F6) {
        settings.show_shared_vertices = !settings.show_shared_vertices;
    }
    if keyboard.just_pressed(KeyCode::F5) {
        settings.show_half_edge_directions = !settings.show_half_edge_directions;
    }
}

pub fn draw_face_topology_debug(
    mut gizmos: Gizmos,
    settings: Res<HexFaceDebugSettings>,
    cache: Res<HexFaceDebugCache>,
    topology: Res<HexFaceTopology>,
    map_data: Res<MapData>,
    phase: Res<State<EditorPhase>>,
) {
    if !debug_overlay_visible(&settings, *phase.get()) || topology.faces.is_empty() {
        return;
    }

    if settings.show_regular_outlines {
        for &coord in map_data.tiles.keys() {
            draw_regular_outline(&mut gizmos, coord);
        }
    }
    if settings.show_warped_outlines {
        draw_warped_outlines(&mut gizmos, &topology, &cache.edges);
    }
    if settings.show_shared_vertices {
        draw_shared_vertices(&mut gizmos, &topology, &cache.shared_vertices);
    }
    if settings.show_half_edge_directions {
        draw_half_edge_directions(&mut gizmos, &topology);
    }
}

fn draw_regular_outline(gizmos: &mut Gizmos, coord: crate::map::HexCoord) {
    let mut points = [Vec3::ZERO; 6];
    for (index, point) in points.iter_mut().enumerate() {
        let key = canonical_corner_key(coord, index);
        let Ok(position) = regular_corner_position(key) else {
            return;
        };
        *point = Vec3::new(position.x, REGULAR_Y_OFFSET, position.y);
    }
    for index in 0..6 {
        gizmos.line(
            points[index],
            points[(index + 1) % 6],
            Color::srgba(0.2, 0.7, 1.0, 0.7),
        );
    }
}

fn draw_warped_outlines(
    gizmos: &mut Gizmos,
    topology: &HexFaceTopology,
    edges: &[UniqueUndirectedEdge],
) {
    for edge in edges {
        let (Some(origin), Some(destination)) = (
            topology.vertices.get(edge.min.index()),
            topology.vertices.get(edge.max.index()),
        ) else {
            continue;
        };
        gizmos.line(
            Vec3::new(origin.position.x, WARPED_Y_OFFSET, origin.position.y),
            Vec3::new(
                destination.position.x,
                WARPED_Y_OFFSET,
                destination.position.y,
            ),
            Color::srgba(1.0, 0.35, 0.1, 0.9),
        );
    }
}

fn draw_shared_vertices(gizmos: &mut Gizmos, topology: &HexFaceTopology, vertices: &[VertexId]) {
    for vertex_id in vertices {
        let Some(vertex) = topology.vertices.get(vertex_id.index()) else {
            continue;
        };
        let center = Vec3::new(vertex.position.x, VERTEX_Y_OFFSET, vertex.position.y);
        let size = 0.08;
        gizmos.line(
            center - Vec3::X * size,
            center + Vec3::X * size,
            Color::srgba(1.0, 0.95, 0.1, 0.95),
        );
        gizmos.line(
            center - Vec3::Z * size,
            center + Vec3::Z * size,
            Color::srgba(1.0, 0.95, 0.1, 0.95),
        );
    }
}

fn draw_half_edge_directions(gizmos: &mut Gizmos, topology: &HexFaceTopology) {
    for (index, edge) in topology.half_edges.iter().enumerate() {
        if index % HALF_EDGE_DRAW_STRIDE != 0 {
            continue;
        }
        let (Some(origin), Some(destination)) = (
            topology.vertices.get(edge.origin.index()),
            topology.vertices.get(edge.destination.index()),
        ) else {
            continue;
        };
        let start = Vec3::new(origin.position.x, ARROW_Y_OFFSET, origin.position.y);
        let end = Vec3::new(
            destination.position.x,
            ARROW_Y_OFFSET,
            destination.position.y,
        );
        let direction = (end - start).normalize_or_zero();
        let tip = end - direction * 0.08;
        let side = Vec3::new(-direction.z, 0.0, direction.x) * 0.05;
        gizmos.line(start, tip, Color::srgba(0.2, 1.0, 0.45, 0.9));
        gizmos.line(
            tip,
            tip - direction * 0.12 + side,
            Color::srgba(0.2, 1.0, 0.45, 0.9),
        );
        gizmos.line(
            tip,
            tip - direction * 0.12 - side,
            Color::srgba(0.2, 1.0, 0.45, 0.9),
        );
    }
}
