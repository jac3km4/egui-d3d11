use std::collections::HashMap;
use egui::{ImageData, TextureId, TexturesDelta};
use parking_lot::{Mutex, MutexGuard};
use windows::Win32::Graphics::{
    Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D,
    Direct3D11::{
        ID3D11Device, ID3D11ShaderResourceView, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE,
        D3D11_RESOURCE_MISC_FLAG, D3D11_SHADER_RESOURCE_VIEW_DESC,
        D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_DYNAMIC,
    },
    Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8_UNORM, DXGI_SAMPLE_DESC},
};

#[allow(dead_code)]
pub struct AllocatedTexture {
    resource: ID3D11ShaderResourceView,
    image: ImageData
}

impl AllocatedTexture {
    #[inline]
    pub fn resource(&self) -> &ID3D11ShaderResourceView {
        &self.resource
    }
}

#[derive(Default)]
pub struct TextureAllocator {
    allocated: Mutex<HashMap<TextureId, AllocatedTexture>>,
}

impl TextureAllocator {
    #[inline]
    pub fn allocated(&self) -> MutexGuard<HashMap<TextureId, AllocatedTexture>> {
        self.allocated.lock()
    }

    #[inline]
    pub fn resolve_delta(&self, delta: TexturesDelta, device: &ID3D11Device) {
        let lock = &mut *self.allocated();

        for free in delta.free {
            drop(lock.remove(&free));
        }

        for (id, img) in delta.set {
            if let Some(_) = img.pos {
                panic!("Partial updates are not supported.");
            } else {
                let tex = Self::allocate_texture(img.image, device);
                lock.insert(id, tex);
            }
        }
    }

    fn allocate_texture(
        image: ImageData,
        device: &ID3D11Device,
    ) -> AllocatedTexture {
        let format = if image.bytes_per_pixel() == 1 {
            DXGI_FORMAT_R8_UNORM
        } else {
            DXGI_FORMAT_R8G8B8A8_UNORM
        };

        let desc = D3D11_TEXTURE2D_DESC {
            Width: image.width() as _,
            Height: image.height() as _,
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Usage: D3D11_USAGE_DYNAMIC,
            BindFlags: D3D11_BIND_SHADER_RESOURCE,
            CPUAccessFlags: D3D11_CPU_ACCESS_WRITE,
            MiscFlags: D3D11_RESOURCE_MISC_FLAG(0),
        };

        let init = D3D11_SUBRESOURCE_DATA {
            pSysMem: match &image {
                ImageData::Color(c) => c.pixels.as_ptr() as _,
                ImageData::Alpha(a) => a.pixels.as_ptr() as _,
            },
            SysMemPitch: (image.width() * image.bytes_per_pixel()) as _,
            SysMemSlicePitch: 0,
        };

        let texture = unsafe {
            expect!(
                device.CreateTexture2D(&desc, &init),
                "Failed to create 2D texture."
            )
        };

        let desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
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
                device.CreateShaderResourceView(&texture, &desc),
                "Failed to create shader resource view."
            )
        };
        drop(texture);

        AllocatedTexture { 
            resource,
            image, 
        }
    }
}
