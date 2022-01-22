use std::mem::size_of;

use egui::{epaint::Vertex, ClippedMesh};
use windows::Win32::Graphics::Direct3D11::{
    ID3D11Buffer, ID3D11Device, D3D11_BIND_INDEX_BUFFER, D3D11_BIND_VERTEX_BUFFER,
    D3D11_BUFFER_DESC, D3D11_SUBRESOURCE_DATA, D3D11_USAGE_DEFAULT,
};

pub struct MeshBuffers {
    pub vertex: ID3D11Buffer,
    pub index: ID3D11Buffer,
}

impl MeshBuffers {
    pub fn new(device: &ID3D11Device, mesh: &ClippedMesh) -> Self {
        Self {
            vertex: Self::create_vertex_buffer(device, mesh),
            index: Self::create_index_buffer(device, mesh),
        }
    }

    fn create_vertex_buffer(device: &ID3D11Device, mesh: &ClippedMesh) -> ID3D11Buffer {
        let buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: (mesh.1.vertices.len() * size_of::<Vertex>()) as _,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_VERTEX_BUFFER,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let init_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: mesh.1.vertices.as_ptr() as _,
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

    fn create_index_buffer(device: &ID3D11Device, mesh: &ClippedMesh) -> ID3D11Buffer {
        let buffer_desc = D3D11_BUFFER_DESC {
            ByteWidth: (mesh.1.indices.len() * size_of::<u32>()) as _,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_INDEX_BUFFER,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let init_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: mesh.1.indices.as_ptr() as _,
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
