use egui::{epaint::Vertex, ClippedMesh, Pos2, Rect, Rgba, TextureId};
use std::mem::size_of;
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11Device, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
    D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
};

/// Egui's [`egui::epaint::Vertex`] uses sRGB colors.
/// I can't be asked to make them work out of the box with hlsl.
/// Color in this vertex uses linear space which I am correcting to gamma in pixel shader.
#[repr(C)]
pub struct GpuVertex {
    pub pos: Pos2,
    pub uv: Pos2,
    pub color: Rgba,
}

impl From<Vertex> for GpuVertex {
    #[inline]
    fn from(v: Vertex) -> Self {
        Self {
            pos: v.pos,
            uv: v.uv,
            color: v.color.into(),
        }
    }
}

#[repr(C)]
pub struct GpuMesh {
    pub vertices: Vec<GpuVertex>,
    pub indices: Vec<u32>,
    pub tex_id: TextureId,
    pub rect: Rect,
}

impl From<ClippedMesh> for GpuMesh {
    #[inline]
    fn from(cm: ClippedMesh) -> Self {
        Self {
            vertices: cm.1.vertices.into_iter().map(GpuVertex::from).collect(),
            tex_id: cm.1.texture_id,
            indices: cm.1.indices,
            rect: cm.0,
        }
    }
}

pub struct MeshBuffers {
    pub vertex: ID3D11Buffer,
    pub index: ID3D11Buffer,
}

impl MeshBuffers {
    pub fn new(device: &ID3D11Device, mesh: &GpuMesh) -> Self {
        Self {
            vertex: Self::create_vertex_buffer(device, mesh),
            index: Self::create_index_buffer(device, mesh),
        }
    }

    fn create_vertex_buffer(device: &ID3D11Device, mesh: &GpuMesh) -> ID3D11Buffer {
        let buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: (mesh.vertices.len() * size_of::<GpuVertex>()) as _,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER.0,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let init_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: mesh.vertices.as_ptr() as _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        unsafe {
            expect!(
                device.CreateBuffer(&buffer_desc, &init_data),
                "Failed to create mesh's vertex buffer"
            )
        }
    }

    fn create_index_buffer(device: &ID3D11Device, mesh: &GpuMesh) -> ID3D11Buffer {
        let buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: (mesh.indices.len() * size_of::<u32>()) as _,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_INDEX_BUFFER.0,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let init_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: mesh.indices.as_ptr() as _,
            SysMemPitch: 0,
            SysMemSlicePitch: 0,
        };

        unsafe {
            expect!(
                device.CreateBuffer(&buffer_desc, &init_data),
                "Failed to create mesh's index buffer"
            )
        }
    }
}

#[inline]
pub fn convert_meshes(clipped: Vec<ClippedMesh>) -> Vec<GpuMesh> {
    clipped
        .into_iter()
        .filter(|m| m.1.indices.is_empty() && m.1.indices.len() % 3 == 0)
        .map(GpuMesh::from)
        .collect()
}
