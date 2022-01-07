use anyhow::Error;
pub mod prelude;

pub type Result<T> = core::result::Result<T, Error>;
