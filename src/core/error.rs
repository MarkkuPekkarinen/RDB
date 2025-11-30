use thiserror::Error;

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum RdbError {
    #[error("Unknown Error")]
    Unknown,
}
