# Spec: Procedural World Generation Overhaul (World-Class Hybrid)

- **Status**: Draft
- **Date**: 2026-05-09
- **Topic**: Hybrid Map Generation (Bones/Carving/Magic)
- **References**: RimWorld, Going Medieval, Whittaker Biome Chart

## 1. Vision & Goals
Create a procedurally generated world that balances "Tactical Intentionality" (mountains, choke points) with "Emergent Chaos" (varied biomes, resources). The system must support seamless cave exploration without loading screens.

## 2. Technical Architecture

### Phase 1: The Bones (Macro Topology)
- **Algorithm**: Voronoi Diagrams combined with low-frequency Fbm Noise.
- **Logic**: Voronoi cells define "Plates". Edges between high-value cells form mountain ranges. Centers of low-value cells form basins/lakes.
- **Elevation**: A multi-tiered heightmap (0-255). 
    - 0-50: Water/Basins
    - 51-150: Plains/Hills
    - 151-255: Mountains/Plateaus

### Phase 2: The Climate (Biomes)
- **Whittaker Chart**: Two independent noise maps: **Temperature** and **Humidity**.
- **Matrix**:
    - Low Temp + High Humid = Swamp / Tundra
    - High Temp + Low Humid = Desert / Savanna
    - Mid Temp + Mid Humid = Temperate Forest / Grassland
- **Hydrography**: "Flow accumulation" logic to place rivers from mountains to lakes. Sand tiles placed near water bodies.

### Phase 3: The Carving (Caves & Stamps)
- **Stamps**: JSON/Asset-based templates for cave rooms (e.g., 8x8 grids).
- **Injection**: Stamps are "melted" into mountain Voronoi cells during startup.
- **Entities**: Stamps include spawners for loot, obstacles, and enemies.

### Phase 4: The Magic (Presentation & UX)
- **Dithered Transparency**: A custom Bevy shader or visibility toggle to hide "Roof" entities when a Pawn is inside a cave zone.
- **Instance Sampling**: Use Bevy 0.18.1's optimized mesh instancing for mass vegetation (trees, bushes).

## 3. Data Structures

```rust
#[derive(Resource)]
pub struct MapData {
    pub width: u32,
    pub height: u32,
    pub tiles: Vec<TileData>,
}

pub struct TileData {
    pub terrain: TerrainType,
    pub elevation: u8,
    pub humidity: f32,
    pub temperature: f32,
    pub is_roofed: bool, // For cave visibility
}
```

## 4. Implementation Plan
1. **Refactor `src/map/mod.rs`**: Move from simple Fbm to a multi-layered generator.
2. **Implement Biome Matrix**: Map Temp/Humidity to `TerrainType`.
3. **Cave System**: Create a basic Stamp loader and "Roof" visibility system.
4. **Visuals**: Update materials to support dithered transparency for mountains.

## 5. Verification
- **Architectural Guard #19**: Ensure UI does not update every frame during gen.
- **Architectural Guard #20**: All gen logic stays in Startup/FixedUpdate.
- **Visual Check**: Mountains should look like ranges, not noise clouds.
