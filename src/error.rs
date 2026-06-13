use std::path::Path;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid Config: {0}: {1}")]
    Config(String, anyhow::Error),
}

impl Error {
    pub fn config<P: AsRef<Path>, E: Into<anyhow::Error>>(path: P, error: E) -> Error {
        Self::Config(path.as_ref().to_string_lossy().into(), error.into())
    }
}
