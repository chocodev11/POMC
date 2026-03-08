use azalea_core::position::ChunkPos;

use crate::renderer::chunk::atlas::{AtlasRegion, TextureAtlas};
use crate::world::block::registry::{BlockRegistry, FaceTextures};
use crate::world::chunk::ChunkStore;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ChunkVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub light: f32,
}

impl ChunkVertex {
    const LAYOUT: &[wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0 => Float32x3,
        1 => Float32x2,
        2 => Float32,
    ];

    pub fn buffer_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::LAYOUT,
        }
    }
}

pub struct ChunkMeshData {
    pub pos: ChunkPos,
    pub vertices: Vec<ChunkVertex>,
    pub indices: Vec<u32>,
}

struct Face {
    positions: [[f32; 3]; 4],
    offset: [i32; 3],
    light: f32,
}

const FACES: [Face; 6] = [
    Face {
        positions: [
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        offset: [0, 1, 0],
        light: 1.0,
    },
    Face {
        positions: [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
        ],
        offset: [0, -1, 0],
        light: 0.5,
    },
    Face {
        positions: [
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        offset: [0, 0, -1],
        light: 0.7,
    },
    Face {
        positions: [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        offset: [0, 0, 1],
        light: 0.7,
    },
    Face {
        positions: [
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
        ],
        offset: [1, 0, 0],
        light: 0.8,
    },
    Face {
        positions: [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
        ],
        offset: [-1, 0, 0],
        light: 0.8,
    },
];

fn face_texture(textures: &FaceTextures, face_idx: usize) -> &str {
    match face_idx {
        0 => textures.top,
        1 => textures.bottom,
        2 => textures.north,
        3 => textures.south,
        4 => textures.east,
        _ => textures.west,
    }
}

pub fn mesh_chunk(
    chunk_store: &ChunkStore,
    pos: ChunkPos,
    registry: &BlockRegistry,
    atlas: &TextureAtlas,
) -> ChunkMeshData {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let min_y = chunk_store.min_y();
    let max_y = min_y + chunk_store.height() as i32;
    let world_x = pos.x * 16;
    let world_z = pos.z * 16;

    for local_z in 0..16i32 {
        for local_x in 0..16i32 {
            let bx = world_x + local_x;
            let bz = world_z + local_z;

            for by in min_y..max_y {
                let state = chunk_store.get_block_state(bx, by, bz);
                if state.is_air() {
                    continue;
                }

                let textures = match registry.get_textures(state) {
                    Some(t) => t,
                    None => continue,
                };

                let fx = bx as f32;
                let fy = by as f32;
                let fz = bz as f32;

                for (i, face) in FACES.iter().enumerate() {
                    let neighbor = chunk_store.get_block_state(
                        bx + face.offset[0],
                        by + face.offset[1],
                        bz + face.offset[2],
                    );
                    if neighbor.is_air() {
                        let region = atlas.get_region(face_texture(textures, i));
                        emit_face(&mut vertices, &mut indices, fx, fy, fz, face, region);
                    }
                }
            }
        }
    }

    ChunkMeshData {
        pos,
        vertices,
        indices,
    }
}

fn emit_face(
    vertices: &mut Vec<ChunkVertex>,
    indices: &mut Vec<u32>,
    bx: f32,
    by: f32,
    bz: f32,
    face: &Face,
    region: AtlasRegion,
) {
    let base = vertices.len() as u32;

    let uvs = [
        [region.u_min, region.v_min],
        [region.u_max, region.v_min],
        [region.u_max, region.v_max],
        [region.u_min, region.v_max],
    ];

    for (i, pos) in face.positions.iter().enumerate() {
        vertices.push(ChunkVertex {
            position: [bx + pos[0], by + pos[1], bz + pos[2]],
            tex_coords: uvs[i],
            light: face.light,
        });
    }

    indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 3, base]);
}
