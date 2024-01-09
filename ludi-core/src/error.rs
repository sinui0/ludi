use std::fmt::Display;

/// Errors that can occur when sending a message.
#[derive(Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MessageError {
    /// The mailbox has been closed.
    Closed,
    /// Handling of the message was interrupted.
    Interrupted,
    /// Error occurred while wrapping a message.
    Wrapper,
}

impl Display for MessageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageError::Closed => write!(f, "mailbox closed"),
            MessageError::Interrupted => write!(f, "message handling interrupted"),
            MessageError::Wrapper => write!(f, "wrapper error"),
        }
    }
}

impl std::error::Error for MessageError {}
