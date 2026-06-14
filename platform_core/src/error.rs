#[derive(Debug)]
pub struct EngineError(pub String);

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for EngineError {}

impl From<String> for EngineError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for EngineError {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}
