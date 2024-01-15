use std::fmt::Display;

/// Errors that can occur when sending a message.
#[derive(Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Error {
    /// The mailbox has been disconnected.
    Disconnected,
    /// Handling of the message was interrupted.
    Interrupted,
    /// Error occurred while wrapping a message.
    Wrapper,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Disconnected => write!(f, "mailbox disconnected"),
            Error::Interrupted => write!(f, "message handling interrupted"),
            Error::Wrapper => write!(f, "wrapper error"),
        }
    }
}

impl std::error::Error for Error {}
