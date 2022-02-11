pub(crate) type Error = Box<dyn std::error::Error>;
pub(crate) type Result<T> = std::result::Result<T, Error>;
