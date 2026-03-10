use thiserror::Error;

#[derive(Debug, Error)]
#[error("{0}")]
pub struct SelfUpdateError(pub(crate) String);

pub type Result<T> = std::result::Result<T, SelfUpdateError>;

pub(crate) fn err(msg: impl Into<String>) -> SelfUpdateError {
    SelfUpdateError(msg.into())
}
