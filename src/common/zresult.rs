pub type ZError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub type ZResult<T> = core::result::Result<T, ZError>;

#[macro_export]
macro_rules! zerror {
    ($msg:expr) => {
        $crate::common::zresult::ZError::from($msg.to_string())
    };

    ($fmt:expr, $($arg:tt)*) => {
        $crate::common::zresult::ZError::from(format!($fmt, $($arg)*))
    };
}
