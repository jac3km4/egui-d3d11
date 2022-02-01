/// This macros allows to hide panicing messages in output binary.
macro_rules! expect {
    ($val:expr, $msg:expr) => {
        if cfg!(feature = "no-msgs") {
            $val.unwrap()
        } else {
            $val.expect($msg)
        }
    };
}

/// Creates zero terminated string.
macro_rules! c_str {
    ($cstr:expr) => {
        windows::Win32::Foundation::PSTR(concat!($cstr, "\x00").as_ptr() as _)
    };
}

mod app;
pub use app::*;

mod texture;
mod backup;
mod shader;
mod mesh;
