use egui::{CtxRef, Pos2, TextureId};
use parking_lot::Mutex;
use std::{
    intrinsics::transmute,
    mem::{size_of, zeroed},
    ptr::null_mut as null,
};
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{HWND, LPARAM, RECT, WPARAM},
        Graphics::{
            Direct3D::D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST,
            Direct3D11::{
                ID3D11Device, ID3D11DeviceContext, ID3D11InputLayout, ID3D11RenderTargetView,
                ID3D11SamplerState, ID3D11Texture2D, D3D11_APPEND_ALIGNED_ELEMENT,
                D3D11_BLEND_DESC, D3D11_BLEND_INV_SRC_ALPHA, D3D11_BLEND_ONE, D3D11_BLEND_OP_ADD,
                D3D11_BLEND_SRC_ALPHA, D3D11_COLOR_WRITE_ENABLE_ALL, D3D11_COMPARISON_ALWAYS,
                D3D11_CULL_NONE, D3D11_FILL_SOLID, D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                D3D11_INPUT_ELEMENT_DESC, D3D11_INPUT_PER_VERTEX_DATA, D3D11_RASTERIZER_DESC,
                D3D11_RENDER_TARGET_BLEND_DESC, D3D11_SAMPLER_DESC, D3D11_TEXTURE_ADDRESS_BORDER,
                D3D11_VIEWPORT,
            },
            Dxgi::{
                Common::{
                    DXGI_FORMAT, DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_R32G32_FLOAT,
                    DXGI_FORMAT_R32_UINT,
                },
                IDXGISwapChain,
            },
        },
        UI::WindowsAndMessaging::GetClientRect,
    },
};

use crate::{
    backup::BackupState,
    input::InputCollector,
    mesh::{convert_meshes, GpuMesh, GpuVertex, MeshBuffers},
    shader::CompiledShaders,
    texture::TextureAllocator,
};

type FnResizeBuffers =
    unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;

#[allow(unused)]
pub struct DirectX11App {
    render_view: Mutex<ID3D11RenderTargetView>,
    input_layout: ID3D11InputLayout,
    tex_alloc: TextureAllocator,
    sampler: ID3D11SamplerState,
    shaders: CompiledShaders,
    input_collector: InputCollector,
    backup: BackupState,
    ctx: Mutex<CtxRef>,
    ui: fn(&CtxRef),
    hwnd: HWND,
}

impl DirectX11App {
    #[inline]
    fn get_screen_size(&self) -> Pos2 {
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut rect);
        }
        Pos2 {
            x: (rect.right - rect.left) as f32,
            y: (rect.bottom - rect.top) as f32,
        }
    }

    const LAYOUT_ELEMENTS: [D3D11_INPUT_ELEMENT_DESC; 3] = [
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: c_str!("POSITION"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: c_str!("TEXCOORD"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        D3D11_INPUT_ELEMENT_DESC {
            SemanticName: c_str!("COLOR"),
            SemanticIndex: 0,
            Format: DXGI_FORMAT_R32G32B32A32_FLOAT,
            InputSlot: 0,
            AlignedByteOffset: D3D11_APPEND_ALIGNED_ELEMENT,
            InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
    ];

    fn create_input_layout(shaders: &CompiledShaders, device: &ID3D11Device) -> ID3D11InputLayout {
        unsafe {
            expect!(
                device.CreateInputLayout(
                    Self::LAYOUT_ELEMENTS.as_ptr(),
                    Self::LAYOUT_ELEMENTS.len() as _,
                    shaders.blobs.vertex.GetBufferPointer(),
                    shaders.blobs.vertex.GetBufferSize()
                ),
                "Failed to create input layout."
            )
        }
    }

    fn create_sampler_state(device: &ID3D11Device) -> ID3D11SamplerState {
        let sampler_desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_BORDER,
            AddressV: D3D11_TEXTURE_ADDRESS_BORDER,
            AddressW: D3D11_TEXTURE_ADDRESS_BORDER,
            MipLODBias: 0.,
            MaxAnisotropy: 1,
            ComparisonFunc: D3D11_COMPARISON_ALWAYS,
            BorderColor: [1., 1., 1., 1.],
            MinLOD: 0.,
            MaxLOD: 0.,
        };

        unsafe {
            expect!(
                device.CreateSamplerState(&sampler_desc),
                "Failed to create sampler state"
            )
        }
    }

    /// Converts texture coords to directx coords which looks like this.
    /// (-1, 1) ============ (1 , 1)
    /// ||                        ||
    /// ||         (0, 0)         ||
    /// ||                        ||
    /// (-1,-1) ============ (1 ,-1)
    fn normalize_meshes(&self, meshes: &mut [GpuMesh]) {
        let mut screen_half = self.get_screen_size();
        screen_half.x /= 2.;
        screen_half.y /= 2.;

        let normalize_point = |point: &mut Pos2| {
            point.x -= screen_half.x;
            point.y -= screen_half.y;

            point.x /= screen_half.x;
            point.y /= -screen_half.y;
        };

        meshes
            .iter_mut()
            .map(|m| (&mut m.vertices, &mut m.rect))
            .for_each(|(vertices, clip)| {
                vertices
                    .iter_mut()
                    .map(|v| &mut v.pos)
                    .for_each(normalize_point);

                normalize_point(&mut clip.min);
                normalize_point(&mut clip.max)
            })
    }

    fn set_blend_state(&self, device: &ID3D11Device, context: &ID3D11DeviceContext) {
        unsafe {
            let mut targets: [D3D11_RENDER_TARGET_BLEND_DESC; 8] = zeroed();
            targets[0].BlendEnable = true.into();
            targets[0].SrcBlend = D3D11_BLEND_SRC_ALPHA;
            targets[0].DestBlend = D3D11_BLEND_INV_SRC_ALPHA;
            targets[0].BlendOp = D3D11_BLEND_OP_ADD;
            targets[0].SrcBlendAlpha = D3D11_BLEND_ONE;
            targets[0].DestBlendAlpha = D3D11_BLEND_INV_SRC_ALPHA;
            targets[0].BlendOpAlpha = D3D11_BLEND_OP_ADD;
            targets[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL as _;

            let blend_desc = D3D11_BLEND_DESC {
                AlphaToCoverageEnable: false.into(),
                IndependentBlendEnable: false.into(),
                RenderTarget: targets,
            };

            let state = expect!(
                device.CreateBlendState(&blend_desc),
                "Failed to create blend state."
            );
            context.OMSetBlendState(&state, [0., 0., 0., 0.].as_ptr(), 0xffffffff);
        }
    }

    fn set_viewports(&self, context: &ID3D11DeviceContext) {
        let size = self.get_screen_size();
        let viewport = D3D11_VIEWPORT {
            TopLeftX: 0.,
            TopLeftY: 0.,
            Width: size.x,
            Height: size.y,
            MinDepth: 0.,
            MaxDepth: 1.,
        };

        unsafe {
            context.RSSetViewports(1, &viewport);
        }
    }

    fn set_raster_state(&self, device: &ID3D11Device, context: &ID3D11DeviceContext) {
        let raster_desc = D3D11_RASTERIZER_DESC {
            FillMode: D3D11_FILL_SOLID,
            CullMode: D3D11_CULL_NONE,
            FrontCounterClockwise: false.into(),
            DepthBias: false.into(),
            DepthBiasClamp: 0.,
            SlopeScaledDepthBias: 0.,
            DepthClipEnable: false.into(),
            ScissorEnable: true.into(),
            MultisampleEnable: false.into(),
            AntialiasedLineEnable: false.into(),
        };

        unsafe {
            let raster_state = expect!(
                device.CreateRasterizerState(&raster_desc),
                "Failed to create rasterizer descriptor"
            );

            context.RSSetState(&raster_state);
        }
    }

    fn create_scissor_rects(meshes: &[GpuMesh]) -> Vec<RECT> {
        meshes
            .iter()
            .map(|m| RECT {
                left: m.rect.left() as _,
                top: m.rect.top() as _,
                right: m.rect.right() as _,
                bottom: m.rect.bottom() as _,
            })
            .collect()
    }

    fn render_meshes(
        &self,
        mut meshes: Vec<GpuMesh>,
        device: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
    ) {
        self.backup.save(ctx);

        // Rects must be created before normalizing.
        let scissor_rects = Self::create_scissor_rects(&meshes);

        self.normalize_meshes(&mut meshes);
        self.set_viewports(ctx);
        self.set_blend_state(device, ctx);
        self.set_raster_state(device, ctx);

        let view_lock = &mut *self.render_view.lock();

        unsafe {
            // context.ClearRenderTargetView(view_lock.clone(), [1., 0., 0., 0.3].as_ptr());

            ctx.OMSetRenderTargets(1, transmute(view_lock), None);
            ctx.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
            ctx.IASetInputLayout(&self.input_layout);

            for mesh in &meshes {
                let buffers = MeshBuffers::new(device, mesh);

                ctx.IASetVertexBuffers(
                    0,
                    1,
                    &Some(buffers.vertex),
                    &(size_of::<GpuVertex>() as _),
                    &0,
                );
                ctx.IASetIndexBuffer(&buffers.index, DXGI_FORMAT_R32_UINT, 0);
                // This doesn't seem to affect anything even if rects are clearly cutting some parts out.
                ctx.RSSetScissorRects(meshes.len() as u32, scissor_rects.as_ptr());

                ctx.VSSetShader(&self.shaders.vertex, null(), 0);
                ctx.PSSetShader(&self.shaders.pixel, null(), 0);
                ctx.PSSetSamplers(0, 1, transmute(&self.sampler));
                ctx.GSSetShader(None, null(), 0);

                if mesh.tex_id == TextureId::Egui {
                    ctx.PSSetShaderResources(0, 1, self.tex_alloc.font_resource());
                }

                ctx.DrawIndexed(mesh.indices.len() as _, 0, 0);
            }
        }

        self.backup.restore(ctx);
    }
}

impl DirectX11App {
    pub fn new(ui: fn(&CtxRef), swap_chain: &IDXGISwapChain, device: &ID3D11Device) -> Self {
        unsafe {
            let hwnd = expect!(
                swap_chain.GetDesc(),
                "Failed to get swapchain's descriptor."
            )
            .OutputWindow;

            if hwnd.is_invalid() {
                if !cfg!(feature = "no-msgs") {
                    panic!("Invalid output window descriptor.");
                } else {
                    unreachable!()
                }
            }

            let back_buffer: ID3D11Texture2D = expect!(
                swap_chain.GetBuffer(0),
                "Failed to get swapchain's back buffer"
            );

            let render_view = expect!(
                device.CreateRenderTargetView(&back_buffer, null()),
                "Failed to create render target view."
            );

            let shaders = CompiledShaders::new(device);

            Self {
                input_layout: Self::create_input_layout(&shaders, device),
                sampler: Self::create_sampler_state(device),
                input_collector: InputCollector::new(hwnd),
                render_view: Mutex::new(render_view),
                ctx: Mutex::new(CtxRef::default()),
                tex_alloc: TextureAllocator::default(),
                backup: BackupState::default(),
                shaders,
                hwnd,
                ui,
            }
        }
    }

    pub fn present(&self, swap_chain: &IDXGISwapChain, _sync_flags: u32, _interval: u32) {
        let (device, context) = get_device_context(swap_chain);

        let ctx_lock = &mut *self.ctx.lock();

        let input = self.input_collector.collect_input();

        let (_output, shapes) = ctx_lock.run(input, self.ui);
        self.tex_alloc
            .update_font_if_needed(&device, &*ctx_lock.font_image());
        let meshes = convert_meshes(ctx_lock.tessellate(shapes));

        self.render_meshes(meshes, &device, &context);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resize_buffers(
        &self,
        swap_chain: &IDXGISwapChain,
        buffer_count: u32,
        width: u32,
        height: u32,
        new_format: DXGI_FORMAT,
        swap_chain_flags: u32,
        original: FnResizeBuffers,
    ) -> HRESULT {
        unsafe {
            let view_lock = &mut *self.render_view.lock();
            std::ptr::drop_in_place(view_lock);

            let result = original(
                swap_chain.clone(),
                buffer_count,
                width,
                height,
                new_format,
                swap_chain_flags,
            );

            let backbuffer: ID3D11Texture2D = expect!(
                swap_chain.GetBuffer(0),
                "Failed to get swapchain's backbuffer."
            );

            let device: ID3D11Device =
                expect!(swap_chain.GetDevice(), "Failed to get swapchain's device.");

            let new_view = expect!(
                device.CreateRenderTargetView(&backbuffer, null()),
                "Failed to create render target view."
            );

            *view_lock = new_view;
            result
        }
    }

    #[inline]
    pub fn wnd_proc(&self, _hwnd: HWND, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> bool {
        self.input_collector.process(umsg, wparam.0, lparam.0);
        true
    }
}

#[inline]
fn get_device_context(swapchain: &IDXGISwapChain) -> (ID3D11Device, ID3D11DeviceContext) {
    unsafe {
        let device: ID3D11Device =
            expect!(swapchain.GetDevice(), "Failed to get swapchain's device");

        let mut context = None;
        device.GetImmediateContext(&mut context);

        (
            device,
            expect!(context, "Failed to get device's immediate context."),
        )
    }
}
