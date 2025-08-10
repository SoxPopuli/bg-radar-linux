use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Memory(String),
    InsufficentMemory {
        msg: String,
        expected: usize,
        actual: usize,
    },
    MissingGameProcess,
    GameProcessClosed,
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
