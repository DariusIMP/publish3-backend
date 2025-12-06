pub type ZError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub type ZResult<T> = core::result::Result<T, ZError>;
