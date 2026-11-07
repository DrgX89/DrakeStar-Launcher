use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum AppError {
    NotFound(String),
    Exists(String),
    Invalid(String),
    Io(String),
    Busy(String),
    Net(String),
}
pub type AppResult<T> = Result<T, AppError>;

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(s) => write!(f, "no encontrado: {s}"),
            Self::Exists(s) => write!(f, "ya existe: {s}"),
            Self::Invalid(s) => write!(f, "{s}"),
            Self::Io(s) => write!(f, "error de E/S: {s}"),
            Self::Busy(s) => write!(f, "{s}"),
            Self::Net(s) => write!(f, "{s}"),
        }
    }
}
impl std::error::Error for AppError {}
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Invalid(format!("JSON invalido: {e}"))
    }
}
impl From<drakestar_core::profile::ProfileError> for AppError {
    fn from(e: drakestar_core::profile::ProfileError) -> Self {
        Self::Invalid(e.to_string())
    }
}
impl From<drakestar_net::NetError> for AppError {
    fn from(e: drakestar_net::NetError) -> Self {
        Self::Net(e.to_string())
    }
}
