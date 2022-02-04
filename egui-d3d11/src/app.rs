use egui::{CtxRef, Pos2, TextureId};
use parking_lot::{Mutex, MutexGuard};
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
                    DXGI_FORMAT_R32G32B32A32_FLOAT, DXGI_FORMAT_R32G32_FLOAT,
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

/// Heart and soul of this integration.
/// Main methods you are going to use are:
/// * [`Self::present`] - Should be called inside of hook are before present.
/// * [`Self::resize_buffers`] - Should be called **INSTEAD** of swapchain's `ResizeBuffers`.
/// * [`Self::wnd_proc`] - Should be called on each `WndProc`. return value doesn't mean anything *yet*.
pub struct DirectX11App<T = ()> {
    ui: Box<dyn FnMut(&CtxRef, &mut T) + 'static>,
    render_view: Mutex<ID3D11RenderTargetView>,
    input_collector: InputCollector,
    input_layout: ID3D11InputLayout,
    tex_alloc: TextureAllocator,
    sampler: ID3D11SamplerState,
    shaders: CompiledShaders,
    backup: BackupState,
    ctx: Mutex<CtxRef>,
    state: Mutex<T>,
    hwnd: HWND,
}

impl<T> DirectX11App<T> {
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

        meshes
            .iter_mut()
            .flat_map(|m| &mut m.vertices)
            .for_each(|v| {
                v.pos.x -= screen_half.x;
                v.pos.y -= screen_half.y;

                v.pos.x /= screen_half.x;
                v.pos.y /= -screen_half.y;
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
            targets[0].RenderTargetWriteMask = D3D11_COLOR_WRITE_ENABLE_ALL.0 as _;

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

    fn render_meshes(
        &self,
        mut meshes: Vec<GpuMesh>,
        device: &ID3D11Device,
        ctx: &ID3D11DeviceContext,
    ) {
        self.backup.save(ctx);

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

            ctx.VSSetShader(&self.shaders.vertex, null(), 0);
            ctx.PSSetShader(&self.shaders.pixel, null(), 0);
            ctx.PSSetSamplers(0, 1, transmute(&self.sampler));
            ctx.GSSetShader(None, null(), 0);

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

                if mesh.tex_id == TextureId::Egui {
                    ctx.PSSetShaderResources(0, 1, self.tex_alloc.font_resource());
                }

                ctx.RSSetScissorRects(
                    1,
                    &RECT {
                        left: (mesh.rect.min.x) as _,
                        top: (mesh.rect.min.y) as _,
                        right: (mesh.rect.max.x) as _,
                        bottom: (mesh.rect.max.y) as _,
                    },
                );

                ctx.DrawIndexed(mesh.indices.len() as _, 0, 0);
            }
        }

        self.backup.restore(ctx);
    }
}

impl<T> DirectX11App<T>
where
    T: Default,
{
    /// Creates new app with state set to default value.
    #[inline]
    pub fn new_with_default(
        ui: impl FnMut(&CtxRef, &mut T) + 'static,
        swap_chain: &IDXGISwapChain,
    ) -> Self {
        Self::new_with_state(ui, swap_chain, T::default())
    }
}

impl<T> DirectX11App<T> {
    /// Returns lock to state of the app.
    pub fn state(&self) -> MutexGuard<T> {
        self.state.lock()
    }

    /// Creates new app with state initialized from closule call.
    #[inline]
    pub fn new_with(
        ui: impl FnMut(&CtxRef, &mut T) + 'static,
        swap_chain: &IDXGISwapChain,
        state: impl FnOnce() -> T,
    ) -> Self {
        Self::new_with_state(ui, swap_chain, state())
    }

    /// Creates new app with explicit state value.
    pub fn new_with_state(
        ui: impl FnMut(&CtxRef, &mut T) + 'static,
        swap_chain: &IDXGISwapChain,
        state: T,
    ) -> Self {
        unsafe {
            let hwnd = expect!(
                swap_chain.GetDesc(),
                "Failed to get swapchain's descriptor."
            )
            .OutputWindow;

            let (device , _) = get_device_context(swap_chain);

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

            let shaders = CompiledShaders::new(&device);

            Self {
                input_layout: Self::create_input_layout(&shaders, &device),
                sampler: Self::create_sampler_state(&device),
                input_collector: InputCollector::new(hwnd),
                render_view: Mutex::new(render_view),
                ctx: Mutex::new(CtxRef::default()),
                tex_alloc: TextureAllocator::default(),
                state: Mutex::new(state),
                backup: BackupState::default(),
                ui: Box::new(ui),
                shaders,
                hwnd,
            }
        }
    }

    /// Present call. Should be called once per original present call, before or inside of hook.
    pub fn present(&self, swap_chain: &IDXGISwapChain, _sync_flags: u32, _interval: u32) {
        let (device, context) = get_device_context(swap_chain);

        let ctx_lock = &mut *self.ctx.lock();

        let input = self.input_collector.collect_input();

        // This should be fine as present can't be called from different threads by
        // a person with enough intelect.
        let ui = self.ui.as_ref() as *const _ as *mut dyn FnMut(&CtxRef, &mut T);
        let (output, shapes) =
            ctx_lock.run(input, |u| unsafe { (*ui)(u, &mut *self.state.lock()) });
        if !output.copied_text.is_empty() {
            // TODO: Do clipboard pasting.
        }

        self.tex_alloc
            .update_font_if_needed(&device, &*ctx_lock.font_image());
        let meshes = convert_meshes(ctx_lock.tessellate(shapes));

        self.render_meshes(meshes, &device, &context);
    }

    /// Call when resizing buffers.
    /// Do not call the original function before it, instead call it inside of the `original` closure.
    #[allow(clippy::too_many_arguments)]
    pub fn resize_buffers(
        &self,
        swap_chain: &IDXGISwapChain,
        original: impl FnOnce() -> HRESULT,
    ) -> HRESULT {
        unsafe {
            let view_lock = &mut *self.render_view.lock();
            std::ptr::drop_in_place(view_lock);

            let result = original();

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

    /// Call on each `WndProc` occurence.
    #[inline]
    pub fn wnd_proc(&self, umsg: u32, wparam: WPARAM, lparam: LPARAM) -> bool {
        self.input_collector.process(umsg, wparam.0, lparam.0);
        true
    }
}

#[inline]
fn get_device_context(swap_chain: &IDXGISwapChain) -> (ID3D11Device, ID3D11DeviceContext) {
    unsafe {
        let device: ID3D11Device =
            expect!(swap_chain.GetDevice(), "Failed to get swapchain's device");

        let mut context = None;
        device.GetImmediateContext(&mut context);

        (
            device,
            expect!(context, "Failed to get device's immediate context."),
        )
    }
}
