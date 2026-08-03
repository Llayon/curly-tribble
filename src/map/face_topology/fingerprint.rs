//! Canonical stable fingerprints for the diagnostic face topology.
//!
//! Fingerprints are computed with an explicit, fixed FNV-1a 64-bit hash over
//! big-endian serialized fields. No `usize`-width dependence, no `HashMap`
//! iteration order, and no diagnostic floating-point metrics (such as
//! `acos`-derived interior angles) are hashed.
use crate::map::data::MapData;
use crate::map::face_topology::types::HexFaceTopology;
use crate::map::WorldSeed;

/// FNV-1a 64-bit offset basis.
pub const FNV1A64_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;

/// FNV-1a 64-bit prime.
pub const FNV1A64_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Computes the FNV-1a 64-bit hash of a byte slice (stable across platforms).
#[must_use]
pub const fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = FNV1A64_OFFSET;
    let mut index = 0;
    while index < bytes.len() {
        hash ^= bytes[index] as u64;
        hash = hash.wrapping_mul(FNV1A64_PRIME);
        index += 1;
    }
    hash
}

/// Immediately sized byte buffer that serializes numeric fields big-endian.
struct FingerprintWriter {
    bytes: Vec<u8>,
}

impl FingerprintWriter {
    fn new(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
        }
    }

    fn push_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn push_u32(&mut self, value: u32) {
        self.push_u64(u64::from(value));
    }

    fn push_usize(&mut self, value: usize) {
        self.push_u64(value as u64);
    }

    fn push_i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    fn push_f32(&mut self, value: f32) {
        self.push_u64(u64::from(value.to_bits()));
    }

    fn finish(self) -> u64 {
        fnv1a64(&self.bytes)
    }
}

/// Geometry and connectivity fingerprints for one canonical fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TopologyFingerprints {
    /// `VertexId` plus X/Y position bits, in ascending `VertexId` order.
    pub geometry: u64,
    /// `FaceId` cycles, `HalfEdge` endpoints, twins, and ownership.
    pub connectivity: u64,
}

/// Computes canonical fingerprints from canonical sorted data.
///
/// The geometry fingerprint covers map dimensions, `WorldSeed`, ascending
/// `VertexId`s and every position component via `f32::to_bits()`. The
/// connectivity fingerprint covers sorted faces (ordered boundary `VertexId`s),
/// sorted half-edges (origin, destination, face owner, and twin identity or
/// absence). Diagnostic `acos`-based metrics are intentionally not hashed.
#[must_use]
pub fn topology_fingerprints(
    map_data: &MapData,
    seed: WorldSeed,
    topology: &HexFaceTopology,
) -> TopologyFingerprints {
    let mut geometry = FingerprintWriter::new(4096);
    geometry.push_u32(map_data.width);
    geometry.push_u32(map_data.height);
    geometry.push_u32(seed.value());

    let mut vertex_ids: Vec<usize> = (0..topology.vertices.len()).collect();
    vertex_ids.sort_unstable();
    for vertex_id in vertex_ids {
        geometry.push_usize(vertex_id);
        let position = topology.vertices[vertex_id].position;
        geometry.push_f32(position.x);
        geometry.push_f32(position.y);
    }

    let mut connectivity = FingerprintWriter::new(16_384);
    connectivity.push_u32(map_data.width);
    connectivity.push_u32(map_data.height);
    connectivity.push_u32(seed.value());

    let mut face_ids: Vec<usize> = (0..topology.faces.len()).collect();
    face_ids.sort_unstable();
    for face_id in face_ids {
        let face = &topology.faces[face_id];
        connectivity.push_usize(face_id);
        connectivity.push_i64(i64::from(face.hex.q));
        connectivity.push_i64(i64::from(face.hex.r));
        for vertex in face.vertices {
            connectivity.push_usize(vertex.index());
        }
    }

    let mut edge_ids: Vec<usize> = (0..topology.half_edges.len()).collect();
    edge_ids.sort_unstable();
    for edge_id in edge_ids {
        let edge = &topology.half_edges[edge_id];
        connectivity.push_usize(edge.origin.index());
        connectivity.push_usize(edge.destination.index());
        connectivity.push_usize(edge.incident_face.index());
        match edge.twin {
            Some(twin_id) => {
                connectivity.push_u64(1);
                connectivity.push_usize(twin_id.index());
            }
            None => connectivity.push_u64(0),
        }
    }

    TopologyFingerprints {
        geometry: geometry.finish(),
        connectivity: connectivity.finish(),
    }
}
