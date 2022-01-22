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

mod app;
pub use app::*;

mod shader;
