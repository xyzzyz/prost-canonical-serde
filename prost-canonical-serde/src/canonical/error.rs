use alloc::string::String;
use core::fmt;

/// Error returned when canonical JSON conversion fails.
#[derive(Debug, Clone)]
pub struct CanonicalError {
    message: String,
}

impl CanonicalError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CanonicalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl core::error::Error for CanonicalError {}
