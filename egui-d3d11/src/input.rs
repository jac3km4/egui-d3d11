use egui::{Event, Modifiers, Pos2, RawInput, Rect};
use parking_lot::Mutex;
use windows::Win32::{
    Foundation::{HWND, RECT},
    System::WindowsProgramming::NtQuerySystemTime,
    UI::WindowsAndMessaging::{GetClientRect, WM_MOUSEMOVE},
};

pub struct InputCollector {
    hwnd: HWND,
    events: Mutex<Vec<Event>>,
}

impl InputCollector {
    pub fn new(hwnd: HWND) -> Self {
        Self {
            hwnd,
            events: Mutex::new(vec![]),
        }
    }

    pub fn process(&self, umsg: u32, _wparam: usize, lparam: isize) {
        let screen = self.get_screen_size();

        match umsg {
            WM_MOUSEMOVE => {
                let mut x = (lparam & 0xFFFF) as f32;
                if x > screen.x { x = 0.; }
                
                let y = (lparam >> 16 & 0xFFFF) as f32;

                self.events.lock().push(Event::PointerMoved(Pos2::new(x, y)));
            }
            _ => {}
        }
    }

    pub fn collect_input(&self) -> RawInput {
        let events = std::mem::take(&mut *self.events.lock());

        RawInput {
            screen_rect: Some(self.get_screen_rect()),
            time: Some(Self::get_system_time()),
            pixels_per_point: Some(1.),
            predicted_dt: 1. / 60.,
            modifiers: Modifiers::default(),
            hovered_files: vec![],
            dropped_files: vec![],
            events,
        }
    }

    pub fn get_system_time() -> f64 {
        let mut time = 0;
        unsafe {
            expect!(NtQuerySystemTime(&mut time), "Failed to get system time.");
        }

        time as _
    }

    #[inline]
    pub fn get_screen_size(&self) -> Pos2 {
        let mut rect = RECT::default();
        unsafe {
            GetClientRect(self.hwnd, &mut rect);
        }

        Pos2::new(
            (rect.right - rect.left) as f32,
            (rect.bottom - rect.top) as f32,
        )
    }

    #[inline]
    pub fn get_screen_rect(&self) -> Rect {
        Rect {
            min: Pos2::ZERO,
            max: self.get_screen_size(),
        }
    }
}
