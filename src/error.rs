use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Could not parse config: {0}: {1}")]
    Config(String, anyhow::Error),
}

impl Error {
    pub fn config<P: Into<String>, E: Into<anyhow::Error>>(path: P, error: E) -> Error {
        Self::Config(path.into(), error.into())
    }
}
