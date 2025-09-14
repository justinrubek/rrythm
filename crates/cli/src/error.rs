#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    ChronoOutOfRange(#[from] chrono::OutOfRangeError),
    #[error(transparent)]
    RRule(#[from] rrule::RRuleError),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
