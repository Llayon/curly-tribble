//! Matrix and architecture guard tests for warped water and roof overlays.

use bevy::prelude::*;

pub struct OverlayMatrixTestsPlugin;

impl Plugin for OverlayMatrixTestsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
    use crate::economy::mesh_gen::generator::create_global_map_meshes;
    use crate::game_state::{EditorPhase, FactionManager};
    use crate::map::data::{OceanState, RoofState};
    use crate::map::face_topology::generate_hex_face_topology_with_profile;
    use crate::map::face_topology::tests_quality_shared::shared as q;
    use crate::map::terrain_gen::TerrainConfig;
    use crate::map::topology::derive_terrain_topology;
    use crate::map::{LandscapeFeature, WorldSeed};
    use bevy::prelude::*;

    #[test]
    fn fast_144_case_overlay_matrix() {
        let mut executed = 0;
        for (_name, shape_map) in q::all_shapes() {
            for profile in q::all_profiles() {
                for seed in q::FAST_SEEDS {
                    let mut map = shape_map.clone();

                    for tile in map.tiles.values_mut() {
                        tile.ocean_state = OceanState::Land;
                        tile.landscape_feature = LandscapeFeature::Lake;
                        tile.roof_state = RoofState::Roofed;
                    }

                    let face_topo = generate_hex_face_topology_with_profile(
                        &map,
                        WorldSeed::new(seed),
                        profile,
                    )
                    .expect("valid face topo");
                    let terr_topo =
                        derive_terrain_topology(&map, &face_topo).expect("valid terrain topo");

                    let (_, water_opt, roof_opt) = create_global_map_meshes(
                        &map,
                        &terr_topo,
                        &face_topo,
                        EditorPhase::Shape,
                        &FactionManager::default(),
                        &TerrainConfig::default(),
                    );

                    let water = water_opt.expect("water mesh expected");
                    let roof = roof_opt.expect("roof mesh expected");

                    let expected_verts = map.tiles.len() * 7;
                    let Some(bevy::mesh::VertexAttributeValues::Float32x3(w_pos)) =
                        water.attribute(Mesh::ATTRIBUTE_POSITION)
                    else {
                        panic!("no water pos");
                    };
                    let Some(bevy::mesh::VertexAttributeValues::Float32x3(r_pos)) =
                        roof.attribute(Mesh::ATTRIBUTE_POSITION)
                    else {
                        panic!("no roof pos");
                    };

                    assert_eq!(w_pos.len(), expected_verts);
                    assert_eq!(r_pos.len(), expected_verts);

                    executed += 1;
                }
            }
        }

        assert_eq!(executed, 144, "fast overlay matrix must run 144 cases");
    }

    #[test]
    fn architecture_guard_no_trigonometry_in_overlay_construction() {
        let source = std::fs::read_to_string("src/economy/mesh_gen/generator.rs")
            .expect("generator.rs must exist");

        assert!(
            !source.contains("angle_deg"),
            "generator.rs must not contain angle_deg"
        );
        assert!(
            !source.contains("angle_rad"),
            "generator.rs must not contain angle_rad"
        );
        assert!(
            !source.contains(".cos()"),
            "generator.rs must not contain .cos()"
        );
        assert!(
            !source.contains(".sin()"),
            "generator.rs must not contain .sin()"
        );
    }
}
