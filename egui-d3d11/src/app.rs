use std::ptr::null_mut as null;
use egui::CtxRef;
use parking_lot::Mutex;
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        Graphics::{
            Direct3D11::{ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, ID3D11RenderTargetView},
            Dxgi::{Common::DXGI_FORMAT, IDXGISwapChain},
        },
    },
};

type FnResizeBuffers =
    unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;

#[allow(unused)]
pub struct DirectX11App {
    render_view: Mutex<ID3D11RenderTargetView>,
    ui: fn(&CtxRef),
    ctx: CtxRef,
    hwnd: HWND,
}

impl DirectX11App {
    pub fn new(ui: fn(&CtxRef), swap_chain: &IDXGISwapChain, device: &ID3D11Device) -> Self {
        unsafe {
            let hwnd = swap_chain.GetDesc().expect("Failed to get swapchain's descriptor.").OutputWindow;
            if hwnd.is_invalid() {
                panic!("Invalid output window descriptor.");
            }

            let back_buffer: ID3D11Texture2D = swap_chain.GetBuffer(0)
                .expect("Failed to get swapchain's back buffer");
            let render_view = device.CreateRenderTargetView(&back_buffer, null())
                .expect("Failed to create render target view.");
            
            Self {
                render_view: Mutex::new(render_view),
                ctx: CtxRef::default(),
                hwnd,
                ui,
            }
        }
    }

    pub fn present(&self, _swap_chain: &IDXGISwapChain, _sync_flags: u32, _interval: u32) { }

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
        eprintln!("Resized");

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
            
            let backbuffer: ID3D11Texture2D = swap_chain.GetBuffer(0)
                .expect("Failed to get swapchain's backbuffer.");

            let device: ID3D11Device = swap_chain.GetDevice().expect("Failed to get swapchain's device.");
            let new_view = device.CreateRenderTargetView(&backbuffer, null())
                .expect("Failed to create render target view.");
            *view_lock = new_view;

            result
        }
    }

    pub fn wnd_proc(&self, _hwnd: HWND, _msg: u32, _wparam: WPARAM, _lparam: LPARAM) -> bool {
        true
    }
}

#[inline]
fn _get_device_context(device: &ID3D11Device) -> ID3D11DeviceContext {
    let mut context = None;
    unsafe { device.GetImmediateContext(&mut context); }
    context.expect("Failed to get device's immediate context.")
}