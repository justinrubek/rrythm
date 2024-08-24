#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("todo")]
    Todo,
    #[error("Failed to build server struct")]
    ServerBuilder,
    #[error(transparent)]
    StdIo(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
