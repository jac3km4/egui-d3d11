use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect};
use parking_lot::Mutex;
use windows::Win32::{
    Foundation::{HWND, RECT},
    System::WindowsProgramming::NtQuerySystemTime,
    UI::{
        Input::KeyboardAndMouse::{
            VK_BACK, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT, VK_LEFT, VK_NEXT,
            VK_PRIOR, VK_RETURN, VK_RIGHT, VK_SPACE, VK_TAB, VK_UP,
        },
        WindowsAndMessaging::{
            GetClientRect, MK_CONTROL, MK_SHIFT, WM_CHAR, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDBLCLK,
            WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDBLCLK, WM_MBUTTONDOWN, WM_MBUTTONUP,
            WM_MOUSEMOVE, WM_RBUTTONDBLCLK, WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN,
            WM_SYSKEYUP,
        },
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
            WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => self.events.lock().push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Secondary,
                pressed: true,
                modifiers: get_modifiers(wparam),
            }),
            WM_RBUTTONUP => self.events.lock().push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Secondary,
                pressed: false,
                modifiers: get_modifiers(wparam),
            }),
            WM_MBUTTONDOWN | WM_MBUTTONDBLCLK => self.events.lock().push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Middle,
                pressed: true,
                modifiers: get_modifiers(wparam),
            }),
            WM_MBUTTONUP => self.events.lock().push(Event::PointerButton {
                pos: get_pos(lparam),
                button: PointerButton::Middle,
                pressed: false,
                modifiers: get_modifiers(wparam),
            }),
            // TODO: Decide if adding `WM_SYSCHAR` is necessary.
            WM_CHAR /* | WM_SYSCHAR */ => {
                if let Some(ch) = char::from_u32(wparam as _) {
                    if ch.is_alphanumeric() {
                        self.events.lock().push(Event::Text(ch.into()));
                    }
                }
            }
            msg @ (WM_KEYDOWN | WM_SYSKEYDOWN) => {
                if let Some(key) = get_key(wparam) {
                    self.events.lock().push(Event::Key {
                        key,
                        pressed: true,
                        modifiers: Modifiers {
                            alt: msg == WM_SYSKEYDOWN,
                            ..Default::default()
                        },
                    });
                }
            }
            msg @ (WM_KEYUP | WM_SYSKEYUP) => {
                if let Some(key) = get_key(wparam) {
                    self.events.lock().push(Event::Key {
                        key,
                        pressed: false,
                        modifiers: Modifiers {
                            alt: msg == WM_SYSKEYDOWN,
                            ..Default::default()
                        },
                    });
                }
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

fn get_key(wparam: usize) -> Option<Key> {
    match wparam {
        0x30..=0x39 => unsafe { Some(std::mem::transmute::<_, Key>(wparam as u8 - 0x21)) },
        0x41..=0x5A => unsafe { Some(std::mem::transmute::<_, Key>(wparam as u8 - 0x28)) },
        _ => match wparam as u16 {
            VK_DOWN => Some(Key::ArrowDown),
            VK_LEFT => Some(Key::ArrowLeft),
            VK_RIGHT => Some(Key::ArrowRight),
            VK_UP => Some(Key::ArrowUp),
            VK_ESCAPE => Some(Key::Escape),
            VK_TAB => Some(Key::Tab),
            VK_BACK => Some(Key::Backspace),
            VK_RETURN => Some(Key::Enter),
            VK_SPACE => Some(Key::Space),
            VK_INSERT => Some(Key::Insert),
            VK_DELETE => Some(Key::Delete),
            VK_HOME => Some(Key::Home),
            VK_END => Some(Key::Home),
            VK_PRIOR => Some(Key::PageUp),
            VK_NEXT => Some(Key::PageDown),
            _ => None,
        },
    }
}
