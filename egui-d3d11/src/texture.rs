use egui::{Color32, ImageData, TextureId, TexturesDelta};
use parking_lot::{Mutex, MutexGuard};
use std::{collections::HashMap, slice::from_raw_parts_mut};
use windows::Win32::Graphics::{
    Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D,
    Direct3D11::{
        ID3D11Device, ID3D11DeviceContext, ID3D11ShaderResourceView, ID3D11Texture2D,
        D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_WRITE, D3D11_MAP_WRITE_DISCARD,
        D3D11_RESOURCE_MISC_FLAG, D3D11_SHADER_RESOURCE_VIEW_DESC,
        D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_DYNAMIC,
    },
    Dxgi::Common::{
        DXGI_FORMAT, DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_FORMAT_R8_UNORM, DXGI_SAMPLE_DESC,
    },
};

pub struct AllocatedTexture {
    resource: ID3D11ShaderResourceView,
    texture: ID3D11Texture2D,
    image: ImageData,
}

impl AllocatedTexture {
    #[inline]
    pub fn resource(&self) -> &ID3D11ShaderResourceView {
        &self.resource
    }

    fn update(&mut self, [x, y]: [usize; 2], delta: ImageData, ctx: &ID3D11DeviceContext) {
        unsafe {
            let subr = ctx
                .Map(&self.texture, 0, D3D11_MAP_WRITE_DISCARD, 0)
                .unwrap();

            match (&self.image, delta) {
                (ImageData::Color(img), ImageData::Color(new)) => {
                    let data = from_raw_parts_mut(
                        subr.pData as *mut Color32,
                        subr.RowPitch as usize * self.image.height(),
                    );
                    data.as_mut_ptr().copy_from_nonoverlapping(
                        img.pixels.as_ptr(),
                        subr.RowPitch as usize * self.image.height(),
                    );

                    let mut i = 0;
                    for y in y..(y + new.height()) {
                        for x in x..(x + new.width()) {
                            data[y * img.width() + x] = new.pixels[i];
                            i += 1;
                        }
                    }
                }
                (ImageData::Alpha(img), ImageData::Alpha(new)) => {
                    let data = from_raw_parts_mut(
                        subr.pData as *mut u8,
                        subr.RowPitch as usize * self.image.height(),
                    );
                    data.as_mut_ptr().copy_from_nonoverlapping(
                        img.pixels.as_ptr(),
                        subr.RowPitch as usize * self.image.height(),
                    );

                    let mut i = 0;
                    for y in y..(y + new.height()) {
                        for x in x..(x + new.width()) {
                            data[y * img.width() + x] = new.pixels[i];
                            i += 1;
                        }
                    }
                }
                _ => unreachable!(),
            }

            ctx.Unmap(&self.texture, 0);
        }
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
    pub fn resolve_delta(
        &self,
        delta: TexturesDelta,
        device: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
    ) {
        let lock = &mut *self.allocated();

        for free in delta.free {
            drop(lock.remove(&free));
        }

        for (id, delta) in delta.set {
            if let Some((region, tex)) = delta.pos.zip(lock.get_mut(&id)) {
                tex.update(region, delta.image, ctx);
            } else {
                let tex = Self::allocate_texture(delta.image, device);
                lock.insert(id, tex);
            }
        }
    }

    fn allocate_texture(image: ImageData, device: &ID3D11Device) -> AllocatedTexture {
        let texture = Self::create_texture(&image, device);
        let resource = Self::create_resource(get_image_format(&image), &texture, device);

        AllocatedTexture {
            resource,
            image,
            texture,
        }
    }

    fn create_texture(image: &ImageData, device: &ID3D11Device) -> ID3D11Texture2D {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: image.width() as _,
            Height: image.height() as _,
            MipLevels: 1,
            ArraySize: 1,
            Format: get_image_format(image),
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
            pSysMem: match image {
                ImageData::Color(c) => c.pixels.as_ptr() as _,
                ImageData::Alpha(a) => a.pixels.as_ptr() as _,
            },
            SysMemPitch: (image.width() * image.bytes_per_pixel()) as _,
            SysMemSlicePitch: 0,
        };

        unsafe {
            expect!(
                device.CreateTexture2D(&desc, &init),
                "Failed to create 2D texture."
            )
        }
    }

    fn create_resource(
        format: DXGI_FORMAT,
        texture: &ID3D11Texture2D,
        device: &ID3D11Device,
    ) -> ID3D11ShaderResourceView {
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

        unsafe {
            expect!(
                device.CreateShaderResourceView(texture, &desc),
                "Failed to create shader resource view."
            )
        }
    }
}

fn get_image_format(image: &ImageData) -> DXGI_FORMAT {
    if image.bytes_per_pixel() == 1 {
        DXGI_FORMAT_R8_UNORM
    } else {
        DXGI_FORMAT_R8G8B8A8_UNORM
    }
}
