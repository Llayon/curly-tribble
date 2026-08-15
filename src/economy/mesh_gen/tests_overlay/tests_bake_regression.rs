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
}
