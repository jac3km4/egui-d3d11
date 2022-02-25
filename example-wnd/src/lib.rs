use egui::{
    Color32, Context, Pos2, Rect, RichText, ScrollArea, Slider, Stroke, TextureId, Vec2, Widget,
};
use egui_d3d11::DirectX11App;
use faithe::{internal::alloc_console, pattern::Pattern};
use std::intrinsics::transmute;
use windows::{
    core::HRESULT,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        Graphics::Dxgi::{Common::DXGI_FORMAT, IDXGISwapChain},
        UI::WindowsAndMessaging::{CallWindowProcW, SetWindowLongPtrA, GWLP_WNDPROC, WNDPROC},
    },
};

#[no_mangle]
unsafe extern "stdcall" fn DllMain(hinst: usize, reason: u32) -> i32 {
    if reason == 1 {
        std::thread::spawn(move || unsafe { main_thread(hinst) });
    }

    1
}

static mut APP: Option<DirectX11App<i32>> = None;
static mut OLD_WND_PROC: Option<WNDPROC> = None;

type FnPresent = unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32) -> HRESULT;
static mut O_PRESENT: Option<FnPresent> = None;

type FnResizeBuffers =
    unsafe extern "stdcall" fn(IDXGISwapChain, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;
static mut O_RESIZE_BUFFERS: Option<FnResizeBuffers> = None;

unsafe extern "stdcall" fn hk_present(
    swap_chain: IDXGISwapChain,
    sync_interval: u32,
    flags: u32,
) -> HRESULT {
    if APP.is_none() {
        APP = Some(DirectX11App::new_with_default(ui, &swap_chain));

        let desc = swap_chain.GetDesc().unwrap();
        if desc.OutputWindow.is_invalid() {
            panic!("Invalid window handle.");
        }
        eprintln!("Buffer fmt: {}", desc.BufferDesc.Format.0);

        OLD_WND_PROC = Some(transmute(SetWindowLongPtrA(
            desc.OutputWindow,
            GWLP_WNDPROC,
            hk_wnd_proc as usize as _,
        )));
    }

    APP.as_ref()
        .unwrap()
        .present(&swap_chain, sync_interval, flags);

    O_PRESENT.as_ref().unwrap()(swap_chain, sync_interval, flags)
}

unsafe extern "stdcall" fn hk_resize_buffers(
    swap_chain: IDXGISwapChain,
    buffer_count: u32,
    width: u32,
    height: u32,
    new_format: DXGI_FORMAT,
    swap_chain_flags: u32,
) -> HRESULT {
    APP.as_ref().unwrap().resize_buffers(&swap_chain, || {
        O_RESIZE_BUFFERS.as_ref().unwrap()(
            swap_chain.clone(),
            buffer_count,
            width,
            height,
            new_format,
            swap_chain_flags,
        )
    })
}

unsafe extern "stdcall" fn hk_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    APP.as_ref().unwrap().wnd_proc(msg, wparam, lparam);

    CallWindowProcW(OLD_WND_PROC.unwrap(), hwnd, msg, wparam, lparam)
}

fn ui(ctx: &Context, i: &mut i32) {
    // You should not use statics like this, it made
    // this way for the sake of example.
    static mut UI_CHECK: bool = true;
    static mut TEXT: Option<String> = None;
    static mut VALUE: f32 = 0.;
    static mut COLOR: [f32; 3] = [0., 0., 0.];

    unsafe {
        if TEXT.is_none() {
            TEXT = Some(String::from("Test"));
        }
    }

    egui::containers::Window::new("Main menu").show(ctx, |ui| {
        ui.label(RichText::new("Test").color(Color32::BLACK));
        ui.label(RichText::new("Other").color(Color32::WHITE));
        ui.separator();

        ui.label(RichText::new(format!("I: {}", *i)).color(Color32::LIGHT_RED));

        unsafe {
            ui.checkbox(&mut UI_CHECK, "Some checkbox");
            ui.text_edit_singleline(TEXT.as_mut().unwrap());
            ScrollArea::vertical().max_height(200.).show(ui, |ui| {
                for i in 1..=100 {
                    ui.label(format!("Label: {}", i));
                }
            });

            Slider::new(&mut VALUE, -1.0..=1.0).ui(ui);

            ui.color_edit_button_rgb(&mut COLOR);
        }

        fn example_plot(ui: &mut egui::Ui) -> egui::Response {
            use egui::plot::{Line, Value, Values};
            let n = 128;
            let line = Line::new(Values::from_values_iter((0..=n).map(|i| {
                use std::f64::consts::TAU;
                let x = egui::remap(i as f64, 0.0..=n as f64, -TAU..=TAU);
                Value::new(x, x.sin())
            })));
            egui::plot::Plot::new("example_plot")
                .height(64.0)
                .data_aspect(1.0)
                .show(ui, |plot_ui| plot_ui.line(line))
                .response
        }

        example_plot(ui);

        ui.label(format!(
            "{:?}",
            &ui.input().pointer.button_down(egui::PointerButton::Primary)
        ));
        if ui.button("You can't click me yet").clicked() {
            *i += 1;
        }
    });

    egui::Window::new("Debug").show(ctx, |ui| {
        unsafe {
            // use `once_cell` crate instead of unsafe code!!!
            static mut IMG: Option<TextureId> = None;
            if IMG.is_none() {
                let s = egui_extras::image::load_image_bytes(include_bytes!("../../logo.bmp")).unwrap();
                IMG = Some(ctx.load_texture("logo", s).id());
            }

            ui.image(IMG.unwrap(), Vec2::new(512., 512.));
        }
    });

    ctx.debug_painter().rect(
        Rect {
            min: Pos2::new(200.0, 200.0),
            max: Pos2::new(300.0, 300.0),
        },
        10.0,
        Color32::from_rgba_premultiplied(255, 0, 0, 150),
        Stroke::none(),
    );

    ctx.debug_painter().circle(
        Pos2::new(350.0, 350.0),
        75.0,
        Color32::from_rgba_premultiplied(0, 255, 0, 200),
        Stroke::none(),
    );
}

unsafe fn main_thread(_hinst: usize) {
    alloc_console().unwrap();

    eprintln!("Hello World!");

    let present = faithe::internal::find_pattern(
        "gameoverlayrenderer64.dll",
        Pattern::from_ida_style("48 89 6C 24 18 48 89 74 24 20 41 56 48 83 EC 20 41"),
    )
    .unwrap_or_else(|_| {
        faithe::internal::find_pattern(
            "dxgi.dll",
            Pattern::from_ida_style("48 89 5C 24 10 48 89 74 24 20 55 57 41 56"),
        )
        .unwrap()
    })
    .unwrap() as usize;

    eprintln!("Present: {:X}", present);

    let swap_buffers = faithe::internal::find_pattern(
        "gameoverlayrenderer64.dll",
        Pattern::from_ida_style(
            "48 89 5C 24 08 48 89 6C 24 10 48 89 74 24 18 57 41 56 41 57 48 83 EC 30 44",
        ),
    )
    .unwrap_or_else(|_| {
        faithe::internal::find_pattern(
            "dxgi.dll",
            Pattern::from_ida_style("48 8B C4 55 41 54 41 55 41 56 41 57 48 8D 68 B1 48 81 EC C0"),
        )
        .unwrap()
    })
    .unwrap() as usize;

    eprintln!("Buffers: {:X}", swap_buffers);

    sunshine::create_hook(
        sunshine::HookType::Compact,
        transmute::<_, FnPresent>(present),
        hk_present as FnPresent,
        &mut O_PRESENT,
    )
    .unwrap();

    sunshine::create_hook(
        sunshine::HookType::Compact,
        transmute::<_, FnResizeBuffers>(swap_buffers),
        hk_resize_buffers as FnResizeBuffers,
        &mut O_RESIZE_BUFFERS,
    )
    .unwrap();

    #[allow(clippy::empty_loop)]
    loop {}
}
