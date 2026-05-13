# Refactor Global Water Mesh Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the global water mesh to follow the tile grid and respect local elevation, allowing rivers to follow carved channels.

**Architecture:** Instead of a single flat plane, the water mesh will be built using the same vertex grid logic as the terrain. It will only include triangles for tiles where the terrain type is `Water`, using the elevation from `MapData`.

**Tech Stack:** Bevy, Rust

---

### Task 1: Refactor Water Mesh Generation

**Files:**
- Modify: `src/economy/mesh_gen.rs`

- [ ] **Step 1: Locate the water mesh generation section**

Find the `// --- ВОДА ---` section in `src/economy/mesh_gen.rs`.

- [ ] **Step 2: Implement grid-based water mesh generation**

Replace the existing `Plane3d` logic with a loop that iterates over tiles and adds vertices and indices for `Water` tiles.

```rust
    // --- ВОДА ---
    let mut water_vertices = Vec::new();
    let mut water_indices = Vec::new();
    let mut water_vertex_count = 0;

    for z in 0..height {
        for x in 0..width {
            let wx = x.cast_signed() - half_w;
            let wz = z.cast_signed() - half_h;

            if let Some(tile) = map.get_tile(wx, wz) {
                if tile.terrain == TerrainType::Water {
                    // Corners for this tile
                    let nw_h = map.get_corner_height(wx, wz);
                    let ne_h = map.get_corner_height(wx + 1, wz);
                    let sw_h = map.get_corner_height(wx, wz + 1);
                    let se_h = map.get_corner_height(wx + 1, wz + 1);

                    let base = water_vertex_count;
                    water_vertices.push([wx as f32, nw_h, wz as f32]);
                    water_vertices.push([(wx + 1) as f32, ne_h, wz as f32]);
                    water_vertices.push([wx as f32, sw_h, (wz + 1) as f32]);
                    water_vertices.push([(wx + 1) as f32, se_h, (wz + 1) as f32]);

                    // Two triangles for the quad
                    // nw, se, ne
                    water_indices.push(base);
                    water_indices.push(base + 3);
                    water_indices.push(base + 1);
                    // nw, sw, se
                    water_indices.push(base);
                    water_indices.push(base + 2);
                    water_indices.push(base + 3);

                    water_vertex_count += 4;
                }
            }
        }
    }

    let mut water_mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    water_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, water_vertices);
    water_mesh.insert_indices(Indices::U32(water_indices));
    water_mesh.compute_normals();
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Success

- [ ] **Step 4: Commit**

```bash
git add src/economy/mesh_gen.rs
git commit -m "refactor: dynamic water mesh that respects local elevation"
```
