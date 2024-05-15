pub use std::process::exit;

#[allow(unused_imports)]
pub use log::error;

#[allow(unused_imports)]
pub use log::warn;

#[allow(unused_imports)]
pub use log::info;

#[allow(unused_imports)]
pub use log::debug;

#[macro_export]
macro_rules! fatal {
    ( $($var:tt)* ) => {
        {
            error!($($var)*);
            exit(1);
        }
    };
}

pub(crate) use fatal;
