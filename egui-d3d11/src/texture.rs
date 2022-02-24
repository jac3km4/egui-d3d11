use std::collections::HashMap;
use egui::{ImageData, TextureId, TexturesDelta};
use parking_lot::{Mutex, MutexGuard};
use windows::Win32::Graphics::{
    Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D,
    Direct3D11::{
        ID3D11Device, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_FLAG, D3D11_RESOURCE_MISC_FLAG,
        D3D11_SHADER_RESOURCE_VIEW_DESC, D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA,
        D3D11_TEX2D_SRV, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11ShaderResourceView,
    },
    Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8_UNORM, DXGI_SAMPLE_DESC, DXGI_FORMAT},
};

#[allow(dead_code)]
pub struct AllocatedTexture {
    view: ID3D11ShaderResourceView,
    format: DXGI_FORMAT
}

#[allow(dead_code)]
impl AllocatedTexture {
    #[inline]
    pub fn resource(&self) -> &ID3D11ShaderResourceView {
        &self.view
    }

    #[inline]
    pub fn is_rgba(&self) -> bool {
        self.format == DXGI_FORMAT_R8G8B8A8_UNORM
    }

    #[inline]
    pub fn is_alpha(&self) -> bool {
        self.format == DXGI_FORMAT_R8_UNORM
    }
}


#[derive(Default)]
pub struct TextureAllocator {
    allocated: Mutex<HashMap<TextureId, AllocatedTexture>>,
}

impl TextureAllocator {
    pub fn allocated(&self) -> MutexGuard<HashMap<TextureId, AllocatedTexture>> {
        self.allocated.lock()
    }

    pub fn resolve_delta(&self, delta: &TexturesDelta, device: &ID3D11Device) {
        for &free in &delta.free {
            self.free_texture(free);
        }

        for (&id, delta) in &delta.set {
            if !delta.is_whole() {
                if cfg!(feature = "no-msgs") {
                    unimplemented!()
                } else {
                    panic!("Partial textures updates are not supported.");
                }
            }

            self.allocate_texture_overwrite(id, &delta.image, device);
        }
    }

    /// Allocates new texture, overwriting the previous one with the same key if was present.
    /// Returns `true` if previous texture was overwritten and `false` if there was no previous texture occupying the `id`.
    pub fn allocate_texture_overwrite(&self, id: TextureId, data: &ImageData, device: &ID3D11Device) -> bool {
        let format = if data.bytes_per_pixel() == 1 {
            DXGI_FORMAT_R8_UNORM
        } else {
            DXGI_FORMAT_R8G8B8A8_UNORM
        };

        let desc = D3D11_TEXTURE2D_DESC {
            Width: data.width() as _,
            Height: data.height() as _,
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE,
            CPUAccessFlags: D3D11_CPU_ACCESS_FLAG(0),
            MiscFlags: D3D11_RESOURCE_MISC_FLAG(0),
        };

        let init_data = D3D11_SUBRESOURCE_DATA {
            pSysMem: match data {
                ImageData::Color(c) => c.pixels.as_ptr() as _,
                ImageData::Alpha(a) => a.pixels.as_ptr() as _,
            },
            SysMemPitch: (data.width() * data.bytes_per_pixel()) as _,
            SysMemSlicePitch: 0,
        };

        let texture = unsafe {
            expect!(
                device.CreateTexture2D(&desc, &init_data),
                "Failed to create image texture."
            )
        };

        let view_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: format,
            ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_SRV {
                    MostDetailedMip: 0,
                    MipLevels: 1,
                },
            },
        };

        let resource = unsafe {
            expect!(
                device.CreateShaderResourceView(&texture, &view_desc),
                "Failed to create shader resource view."
            )
        };
        drop(texture);

        let tex = AllocatedTexture {
            view: resource,
            format,
        };

        if let Some(_) = self.allocated.lock().insert(id, tex) {
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    /// Returns `true` if new texture was successfully allocated or`false` if such key was already present.
    pub fn allocate_texture_if_needed(&self, id: TextureId, data: &ImageData, device: &ID3D11Device) -> bool {
        if self.allocated.lock().contains_key(&id) {
            return false;
        }

        self.allocate_texture_overwrite(id, data, device);

        true
    }

    /// Returns `true` if texture was dropped or `false` if this ID was not present.
    pub fn free_texture(&self, id: TextureId) -> bool {
        if let Some(_) = self.allocated.lock().remove(&id) {
            true
        } else {
            false
        }
    }
}
