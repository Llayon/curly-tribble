//! Bit-regression: water and roof overlays produced by the M5.1 bake renderer
//! must be bit-identical to the legacy generator's overlays, because both share
//! `build_water_and_roof_meshes`. Ground geometry is allowed to differ (bake is
//! authoritative); overlays must not regress.

use bevy::prelude::*;

pub struct OverlayBakeRegressionTestsPlugin;

impl Plugin for OverlayBakeRegressionTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::economy::mesh_gen::bake::create_global_map_meshes_from_bake;
    use crate::economy::mesh_gen::generator::create_global_map_meshes;
    use crate::game_state::{EditorPhase, FactionManager};
    use crate::map::data::{MapData, OceanState, RoofState, TileData};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::profiles::HexDeformationProfile;
    use crate::map::height_constraints::compile_height_constraints;
    use crate::map::height_graph::builder::build_height_constraint_graph;
    use crate::map::surface_height::guide::derive_legacy_height_guide;
    use crate::map::surface_height::hard_constraints::compile_hard_constraints;
    use crate::map::surface_height::solver::solve_surface_heights;
    use crate::map::surface_height::targets::compile_height_targets;
    use crate::map::surface_height::types::HeightSolverConfig;
    use crate::map::surface_topology::generate_surface_topology;
    use crate::map::terrain_bake::builder::build_surface_terrain_bake;
    use crate::map::terrain_gen::TerrainConfig;
    use crate::map::topology::derive_terrain_topology;
    use crate::map::{HexCoord, LandscapeFeature, WorldSeed};
    use bevy::prelude::*;

    fn build_map_with_water_and_roof() -> MapData {
        let mut map = MapData::default();

        let lake_hex = HexCoord::new(0, 0);
        map.tiles.insert(
            lake_hex,
            TileData {
                ocean_state: OceanState::Land,
                landscape_feature: LandscapeFeature::Lake,
                elevation: 0.4,
                ..Default::default()
            },
        );

        let roof_hex = HexCoord::new(1, 0);
        map.tiles.insert(
            roof_hex,
            TileData {
                ocean_state: OceanState::Land,
                roof_state: RoofState::Roofed,
                elevation: 0.7,
                ..Default::default()
            },
        );

        map
    }

    fn build_bake(map: &MapData) -> crate::map::terrain_bake::types::SurfaceTerrainBake {
        let seed = WorldSeed::new(42);
        let config = HeightSolverConfig::default();
        let face_top =
            generate_hex_face_topology_with_profile(map, seed, HexDeformationProfile::Subtle)
                .expect("face topology");
        let surface = generate_surface_topology(&face_top).expect("surface topology");
        let constraints = compile_height_constraints(map, &surface).expect("height constraints");
        let graph = build_height_constraint_graph(&surface, &constraints).expect("height graph");
        let guide = derive_legacy_height_guide(map, &surface, &graph).expect("height guide");
        let targets = compile_height_targets(&graph, &guide, &config).expect("height targets");
        let hard = compile_hard_constraints(&graph, &guide, &config).expect("hard constraints");
        let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config).expect("solve");
        build_surface_terrain_bake(&surface, &graph, &layer).expect("terrain bake")
    }

    /// Renders via BOTH paths and asserts water/roof overlays are bit-identical.
    #[test]
    fn bake_path_water_roof_bit_matches_legacy() {
        let map = build_map_with_water_and_roof();
        let seed = WorldSeed::new(42);
        let config = HeightSolverConfig::default();
        let factions = FactionManager::default();
        let terrain_config = TerrainConfig::default();

        let face_top =
            generate_hex_face_topology_with_profile(&map, seed, HexDeformationProfile::Subtle)
                .expect("face topology");
        let surface = generate_surface_topology(&face_top).expect("surface topology");
        let constraints = compile_height_constraints(&map, &surface).expect("height constraints");
        let graph = build_height_constraint_graph(&surface, &constraints).expect("height graph");
        let guide = derive_legacy_height_guide(&map, &surface, &graph).expect("height guide");
        let targets = compile_height_targets(&graph, &guide, &config).expect("height targets");
        let hard = compile_hard_constraints(&graph, &guide, &config).expect("hard constraints");
        let layer = solve_surface_heights(&graph, &guide, &targets, &hard, &config).expect("solve");
        let bake = build_surface_terrain_bake(&surface, &graph, &layer).expect("terrain bake");

        let terrain_topo = derive_terrain_topology(&map, &face_top).expect("terrain topology");

        let phase = EditorPhase::Height3D;

        let (_, legacy_water, legacy_roof) = create_global_map_meshes(
            &map,
            &terrain_topo,
            &face_top,
            phase,
            &factions,
            &terrain_config,
        )
        .expect("legacy render");

        let (_, bake_water, bake_roof) = create_global_map_meshes_from_bake(
            &map,
            &bake,
            &face_top,
            phase,
            &factions,
            &terrain_config,
            &crate::map::surface_gameplay::types::SurfaceGameplayMap::default(),
        )
        .expect("bake render");

        assert!(
            legacy_water.is_some(),
            "map must contain a water overlay (lake tile)"
        );
        assert!(
            legacy_roof.is_some(),
            "map must contain a roof overlay (roofed tile)"
        );

        assert_meshes_bit_identical(&legacy_water, &bake_water, "water");
        assert_meshes_bit_identical(&legacy_roof, &bake_roof, "roof");
    }

    fn assert_meshes_bit_identical(legacy: &Option<Mesh>, bake: &Option<Mesh>, name: &str) {
        match (legacy, bake) {
            (None, None) => {}
            (Some(a), Some(b)) => {
                let Some(bevy::mesh::VertexAttributeValues::Float32x3(ap)) =
                    a.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    panic!("{name}: missing legacy positions");
                };
                let Some(bevy::mesh::VertexAttributeValues::Float32x3(bp)) =
                    b.attribute(Mesh::ATTRIBUTE_POSITION)
                else {
                    panic!("{name}: missing bake positions");
                };
                assert_eq!(
                    ap.len(),
                    bp.len(),
                    "{name}: vertex count mismatch legacy vs bake"
                );
                for (i, (pa, pb)) in ap.iter().zip(bp).enumerate() {
                    for c in 0..3 {
                        assert_eq!(
                            pa[c].to_bits(),
                            pb[c].to_bits(),
                            "{name} vertex {i} component {c}: bit mismatch"
                        );
                    }
                }
                let Some(bevy::mesh::Indices::U32(ai)) = a.indices() else {
                    panic!("{name}: missing legacy indices");
                };
                let Some(bevy::mesh::Indices::U32(bi)) = b.indices() else {
                    panic!("{name}: missing bake indices");
                };
                assert_eq!(ai, bi, "{name}: index mismatch legacy vs bake");
            }
            _ => panic!("{name}: overlay presence mismatch between legacy and bake paths"),
        }
    }

    /// Buildability colors must come from the SurfaceGameplayMap (policy
    /// external) whenever the build-area layer is visible in Sediments phase:
    /// land buildable -> green, land non-buildable -> red, ocean -> blue.
    #[test]
    fn buildability_overlay_uses_surface_gameplay_map() {
        let mut map = MapData::default();
        let green_hex = HexCoord::new(0, 0);
        let red_hex = HexCoord::new(1, 0);
        let ocean_hex = HexCoord::new(2, 0);
        for (hex, ocean_state, elevation) in [
            (green_hex, OceanState::Land, 0.4),
            (red_hex, OceanState::Land, 0.4),
            (ocean_hex, OceanState::Ocean, 0.0),
        ] {
            map.tiles.insert(
                hex,
                TileData {
                    ocean_state,
                    elevation,
                    ..Default::default()
                },
            );
        }
        let bake = build_bake(&map);
        let face_top = generate_hex_face_topology_with_profile(
            &map,
            WorldSeed::new(42),
            HexDeformationProfile::Subtle,
        )
        .expect("face topology");

        let mut gameplay = crate::map::surface_gameplay::types::SurfaceGameplayMap::default();
        gameplay.cells.insert(
            green_hex,
            crate::map::surface_gameplay::types::SurfaceGameplayCell {
                buildable: true,
                ..Default::default()
            },
        );
        gameplay.cells.insert(
            red_hex,
            crate::map::surface_gameplay::types::SurfaceGameplayCell {
                buildable: false,
                ..Default::default()
            },
        );

        let mut terrain_config = TerrainConfig::default();
        terrain_config.build_area_layer = crate::map::terrain_gen::LayerVisibility::Visible;

        let (mesh, _, _) = create_global_map_meshes_from_bake(
            &map,
            &bake,
            &face_top,
            EditorPhase::Sediments,
            &FactionManager::default(),
            &terrain_config,
            &gameplay,
        )
        .expect("bake render");

        let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
            mesh.attribute(Mesh::ATTRIBUTE_COLOR)
        else {
            panic!("missing color attribute");
        };
        let green = [0.2, 1.0, 0.2, 1.0];
        let red = [1.0, 0.25, 0.25, 1.0];
        let blue = [0.1, 0.4, 0.9, 1.0];

        for (vertex, color) in bake.vertices.iter().zip(colors) {
            let owns = |hex| vertex.owner_hexes.contains(&hex);
            let (coord_name, expected) = if owns(green_hex) {
                ("green", green)
            } else if owns(red_hex) {
                ("red", red)
            } else if owns(ocean_hex) {
                ("ocean", blue)
            } else {
                continue;
            };
            assert_eq!(
                *color, expected,
                "vertex {vertex:?} (owner {coord_name}) must be {coord_name}"
            );
        }
    }
}
