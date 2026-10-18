#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureClass {
    Retryable,
    NotRetryable,
    Transient,
}

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct RuntimeError {
    pub class: FailureClass,
    pub code: String,
    pub message: String,
    pub resets_at: Option<i64>,
}

impl RuntimeError {
    pub fn new(class: FailureClass, code: &str, message: impl Into<String>) -> RuntimeError {
        RuntimeError {
            class,
            code: code.to_owned(),
            message: message.into(),
            resets_at: None,
        }
    }

    pub fn resetting_at(self, resets_at: Option<i64>) -> RuntimeError {
        RuntimeError { resets_at, ..self }
    }
}
