use egui::FontImage;
use std::cell::Cell;
use windows::Win32::Graphics::{
    Direct3D::D3D11_SRV_DIMENSION_TEXTURE2D,
    Direct3D11::{
        ID3D11Device, ID3D11ShaderResourceView, D3D11_BIND_SHADER_RESOURCE, D3D11_CPU_ACCESS_FLAG,
        D3D11_RESOURCE_MISC_FLAG, D3D11_SHADER_RESOURCE_VIEW_DESC,
        D3D11_SHADER_RESOURCE_VIEW_DESC_0, D3D11_SUBRESOURCE_DATA, D3D11_TEX2D_SRV,
        D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT,
    },
    Dxgi::Common::{DXGI_FORMAT_R8_UNORM, DXGI_SAMPLE_DESC},
};

#[derive(Default)]
pub struct TextureAllocator {
    font: Cell<Option<ID3D11ShaderResourceView>>,
    hash: Cell<u64>,
}

impl TextureAllocator {
    pub fn font_resource(&self) -> *const Option<ID3D11ShaderResourceView> {
        self.font.as_ptr() as _
    }

    pub fn update_font_if_needed(&self, device: &ID3D11Device, font: &FontImage) {
        if font.version == self.hash.get() {
            return;
        }

        let tex_desc = D3D11_TEXTURE2D_DESC {
            Width: font.width as u32,
            Height: font.height as u32,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8_UNORM,
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
            pSysMem: font.pixels.as_ptr() as _,
            SysMemPitch: font.width as u32,
            SysMemSlicePitch: 0,
        };

        unsafe {
            let font_tex = expect!(
                device.CreateTexture2D(&tex_desc, &init_data),
                "Failed to create font texture."
            );

            let resource_view_desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
                Format: DXGI_FORMAT_R8_UNORM,
                ViewDimension: D3D11_SRV_DIMENSION_TEXTURE2D,
                Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                    Texture2D: D3D11_TEX2D_SRV {
                        MostDetailedMip: 0,
                        MipLevels: 1,
                    },
                },
            };

            let font_resource = expect!(
                device.CreateShaderResourceView(&font_tex, &resource_view_desc),
                "Failed to create font shader resource."
            );

            self.font.set(Some(font_resource));
            self.hash.set(font.version);
        }
    }
}
