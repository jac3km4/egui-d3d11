use egui::{Event, Modifiers, PointerButton, Pos2, RawInput, Rect};
use parking_lot::Mutex;
use windows::Win32::{
    Foundation::{HWND, RECT},
    System::WindowsProgramming::NtQuerySystemTime,
    UI::WindowsAndMessaging::{
        GetClientRect, MK_CONTROL, MK_SHIFT, WM_LBUTTONDBLCLK, WM_LBUTTONDOWN, WM_LBUTTONUP,
        WM_MOUSEMOVE,
    },
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

    pub fn process(&self, umsg: u32, wparam: usize, lparam: isize) {
        match umsg {
            WM_MOUSEMOVE => self
                .events
                .lock()
                .push(Event::PointerMoved(get_pos(lparam))),
            WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => self.events.lock().push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Primary,
                pressed: true,
                modifiers: get_modifiers(wparam),
            }),
            WM_LBUTTONUP => self.events.lock().push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Primary,
                pressed: false,
                modifiers: get_modifiers(wparam),
            }),
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

    /// Returns time in seconds.
    pub fn get_system_time() -> f64 {
        let mut time = 0;
        unsafe {
            expect!(NtQuerySystemTime(&mut time), "Failed to get system time.");
        }

        // dumb ass, read the docs. egui clearly says `in seconds`.
        // Shouldn't have wasted 3 days on this.
        // `NtQuerySystemTime` returns how many 100 nanosecond intervals
        // past since 1st Jan, 1601.
        (time as f64) / 10_000_000.
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

fn get_pos(lparam: isize) -> Pos2 {
    let x = (lparam & 0xFFFF) as i16 as f32;
    let y = (lparam >> 16 & 0xFFFF) as i16 as f32;

    Pos2::new(x, y)
}

fn get_modifiers(wparam: usize) -> Modifiers {
    Modifiers {
        alt: false,
        ctrl: (wparam & MK_CONTROL as usize) != 0,
        shift: (wparam & MK_SHIFT as usize) != 0,
        mac_cmd: false,
        command: false,
    }
}
