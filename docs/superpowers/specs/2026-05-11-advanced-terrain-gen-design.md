# Spec: Advanced Terrain Generation

- **Status**: Draft
- **Date**: 2026-05-11
- **Topic**: Procedural Terrain with Ridges and Terraces
- **References**: noise crate, Bevy 0.18

## 1. Vision & Goals
Implement a high-fidelity terrain generation function that creates "playable" environments: sharp mountain ridges for natural borders, open passes for chokepoints, and terraced plateaus for settlement building.

## 2. Technical Architecture

### Noise Channels
- **Macro Ridges**: `Fbm<Perlin>` with inversion `1.0 - abs(x)` and power scale `x^4`.
- **Pass Mask**: Low-frequency `Perlin` to carve openings in the ridges.
- **Plateau Base**: `Fbm<Perlin>` for the foundation.
- **Domain Warp**: Two `Perlin` channels (X, Z) to distort plateau coordinates.

### Math Logic
1. **Ridged Mountain**: 
   `m = (1.0 - abs(noise(x * freq))).pow(sharpness)`
   `pass = (noise(x * pass_freq) + 1) / 2`
   `macro = m * pass * MACRO_HEIGHT`
2. **Terraced Plateau**:
   `nx, nz = x + warp_x(x, z), z + warp_z(x, z)`
   `base = (noise(nx, nz) + 1) / 2`
   `stepped = smoothstep_terracing(base, STEPS)`
   `micro = stepped * PLATEAU_HEIGHT`
3. **Blending**:
   `elevation = max(macro, micro)`

### Bevy Integration
- `TerrainGenerator` as a `Resource`.
- Initialized in a startup system with a `WorldSeed`.

## 3. Configuration (Constants)
- `MACRO_FREQ`: 0.03
- `PASS_FREQ`: 0.01
- `MACRO_HEIGHT`: 12.0
- `PLATEAU_FREQ`: 0.04
- `PLATEAU_HEIGHT`: 4.0
- `PLATEAU_STEPS`: 4.0
- `WARP_STRENGTH`: 5.0

## 4. Verification
- **Visual**: Ridges should look like continuous chains with occasional breaks.
- **Functional**: Plateaus should have flat areas and distinct walkable slopes.
- **Performance**: Ensure no redundant noise instance creation.
