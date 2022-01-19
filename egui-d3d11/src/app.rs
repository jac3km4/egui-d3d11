use egui::CtxRef;
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        Graphics::{
            Direct3D11::ID3D11Device,
            Dxgi::{Common::DXGI_FORMAT, IDXGISwapChain},
        },
    },
};

type FnResizeBuffers =
    unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;

#[allow(unused)]
pub struct DirectX11App {
    ctx: CtxRef,
    ui: fn(&CtxRef),
}

impl DirectX11App {
    pub fn new(ui: fn(&CtxRef), _swap_chain: &IDXGISwapChain, _device: &ID3D11Device) -> Self {
        Self {
            ctx: CtxRef::default(),
            ui,
        }
    }

    pub fn present(&self, _swap_chain: &IDXGISwapChain, _sync_flags: u32, _interval: u32) {}

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
            original(
                swap_chain.clone(),
                buffer_count,
                width,
                height,
                new_format,
                swap_chain_flags,
            )
        }
    }

    pub fn wnd_proc(&self, _hwnd: HWND, _msg: u32, _wparam: WPARAM, _lparam: LPARAM) -> bool {
        true
    }
}
