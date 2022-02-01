use egui::{Pos2, RawInput, Rect, Modifiers};
use windows::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::GetClientRect, System::WindowsProgramming::NtQuerySystemTime,
};

pub struct InputCollector {
    hwnd: HWND,
}

impl InputCollector {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd }
    }

    pub fn process(&self, _umsg: u32, _wparam: usize, _lparam: isize) {
        // TODO
    }

    pub fn collect_input(&self) -> RawInput {
        RawInput {
            screen_rect: Some(self.get_client_rect()),
            time: Some(Self::get_system_time()),
            pixels_per_point: Some(1.),
            predicted_dt: 1. / 60.,
            modifiers: Modifiers::default(),
            events: vec![],
            hovered_files: vec![],
            dropped_files: vec![],
        }
    }

    pub fn get_system_time() -> f64 {
        let mut time = 0;
        unsafe { 
            expect!(
                NtQuerySystemTime(&mut time),
                "Failed to get system time."
            );
        }

        time as _
    }

    pub fn get_client_rect(&self) -> Rect {
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut rect);
        }

        Rect {
            min: Pos2::ZERO,
            max: Pos2::new(
                (rect.right - rect.left) as f32,
                (rect.bottom - rect.top) as f32
            ),
        }
    }
}
