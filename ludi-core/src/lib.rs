//! Core types and traits for the ludi library.

#![deny(unsafe_code)]
#![deny(unused_must_use)]
#![deny(missing_docs)]
#![deny(unreachable_pub)]
#![deny(clippy::all)]

mod address;
mod envelope;
mod error;
mod mailbox;

use futures_util::StreamExt;
use std::future::Future;

pub use address::Address;
pub use envelope::Envelope;
pub use error::MessageError;
pub use mailbox::{IntoMail, IntoMailbox, Mailbox};

#[cfg(feature = "futures-mailbox")]
pub use address::futures_address::FuturesAddress;
#[cfg(feature = "futures-mailbox")]
pub use mailbox::futures_mailbox::{mailbox, FuturesMailbox};

/// A message type.
pub trait Message: Send + 'static {
    /// The return value of the message.
    type Return: Send + 'static;
}

/// A message which can wrap another type of message.
pub trait Wrap<T: Message>: From<T> + Message {
    /// Unwraps the return value of the message.
    fn unwrap_return(ret: Self::Return) -> Result<T::Return, MessageError>;
}

impl<T: Message> Wrap<T> for T {
    fn unwrap_return(ret: Self::Return) -> Result<T::Return, MessageError> {
        Ok(ret)
    }
}

/// A message which can be dispatched to an actor.
pub trait Dispatch<A: Actor>: Message {
    /// Dispatches the message and return channel to the actor for handling.
    ///
    /// # Arguments
    ///
    /// * `actor` - The actor which will handle the message.
    /// * `ctx` - The context of the actor.
    /// * `ret` - A channel which returns a value to the caller.
    fn dispatch<R: FnOnce(Self::Return) + Send>(
        self,
        actor: &mut A,
        ctx: &mut Context<A>,
        ret: R,
    ) -> impl Future<Output = ()> + Send;
}

/// An actor.
///
/// # Message Type
///
/// Each actor must specify the type of message it can handle. See the [`Message`] trait for
/// more information and examples.
///
/// # Start
///
/// When an actor is first started, the [`Actor::started`] method will be called. By default this method
/// does nothing, but it can be overridden to perform any initialization required by the actor.
///
/// # Stop
///
/// When an actor receives a stop signal it will stop processing messages and the [`Actor::stopped`] method
/// will be called before returning.
///
/// # Example
///
/// ```ignore
/// # use ludi_core::*;
/// struct PingActor;
///
/// impl Actor for PingActor {
///     type Stop = ();
///
///     async fn stopped(&mut self) -> Self::Stop {
///        println!("actor stopped");
///     }
/// }
/// ```
pub trait Actor: Send + Sized + 'static {
    /// The type of value returned when this actor is stopped.
    type Stop;
    /// The type of error which may occur during handling.
    type Error: Send + 'static;

    /// A method which can be overridden to perform any initialization required by the
    /// actor during startup.
    fn started(&mut self, _ctx: &mut Context<Self>) -> Result<(), Self::Error> {
        Ok(())
    }

    /// A method which is called when the actor receives a stop signal.
    fn stopped(&mut self) -> impl Future<Output = Result<Self::Stop, Self::Error>> + Send;
}

/// An actor that can handle a message.
///
/// When an actor receives a message it is passed to its' handler which
/// processes the message and optionally returns a value to the caller.
///
/// For extra control over how a message is handled, see [`Handler::process`].
pub trait Handler<T: Message>: Actor {
    /// Handle a message and return a value to the caller.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to handle.
    /// * `ctx` - The actor's execution context.
    fn handle(&mut self, msg: T, ctx: &mut Context<Self>)
        -> impl Future<Output = T::Return> + Send;

    /// Handle a message and return a value to the caller. This method is similar to [`Handler::handle`]
    /// except that it gives more control over how the message is handled.
    ///
    /// By default, this method simply calls [`Handler::handle`] and returns the value back to the caller.
    ///
    /// # Arguments
    ///
    /// * `msg` - The message to handle.
    /// * `ctx` - The actor's execution context.
    /// * `ret` - A channel which returns a value to the caller.
    ///
    /// # Defer handling
    ///
    /// Ownership of the return channel `ret` is provided to this method. This allows the
    /// actor to defer handling of the message until later, or to send the message to another
    /// thread for processing without blocking the actor.
    ///
    /// # Post processing
    ///
    /// It may be useful to perform post-processing after a message has been handled. This can be
    /// done by overriding this method and performing work after the value has been sent back to
    /// the caller.
    fn process<R: FnOnce(T::Return) + Send>(
        &mut self,
        msg: T,
        ctx: &mut Context<Self>,
        ret: R,
    ) -> impl Future<Output = ()> + Send {
        async move { ret(self.handle(msg, ctx).await) }
    }
}

/// A controller for an actor.
pub trait Controller {
    /// The type of actor that this controller controls.
    type Actor: Actor;
    /// The type of address that this controller uses to send messages to the actor.
    type Address: Address<Message = Self::Message>;
    /// The type of message that this controller can send to the actor.
    type Message: Message + Dispatch<Self::Actor>;

    /// Returns the address of the actor.
    fn address(&self) -> &Self::Address;
}

/// An actor's execution context.
pub struct Context<A: Actor> {
    stopped: bool,
    err: Option<A::Error>,
}

impl<A: Actor> Default for Context<A> {
    fn default() -> Self {
        Self {
            stopped: Default::default(),
            err: Default::default(),
        }
    }
}

impl<A: Actor> Context<A> {
    /// Signals to the actor that it should stop processing messages.
    pub fn stop(&mut self) {
        self.stopped = true;
    }

    /// Returns `true` if the actor has received a stop signal.
    pub fn stopped(&self) -> bool {
        self.stopped
    }

    /// Returns an error if one has occurred.
    pub fn error(&self) -> Option<&A::Error> {
        self.err.as_ref()
    }

    /// Propagates an error to the actor context.
    pub fn set_error(&mut self, err: A::Error) {
        self.err = Some(err);
    }

    /// Takes an error if one has occurred.
    pub fn take_error(&mut self) -> Option<A::Error> {
        self.err.take()
    }

    /// Executes a fallible function and propagates any errors to the context.
    pub async fn try_or_stop<
        F: FnOnce(&mut Self) -> Fut,
        Fut: Future<Output = Result<Ok, A::Error>>,
        Ok,
    >(
        &mut self,
        f: F,
    ) -> Option<Ok> {
        match f(self).await {
            Ok(ok) => Some(ok),
            Err(err) => {
                self.err = Some(err);
                None
            }
        }
    }
}

/// Runs an actor until it receives a stop signal or an error occurs.
///
/// # Arguments
///
/// * `actor` - The actor to run.
/// * `mailbox` - The mailbox which will be used to receive messages.
pub async fn run<A, M>(actor: &mut A, mailbox: &mut M) -> Result<A::Stop, A::Error>
where
    A: Actor,
    M: Mailbox,
    M::Message: Dispatch<A>,
{
    let mut ctx = Context::default();
    actor.started(&mut ctx)?;

    while let Some(env) = mailbox.next().await {
        env.dispatch(actor, &mut ctx).await;

        if let Some(err) = ctx.take_error() {
            return Err(err);
        } else if ctx.stopped() {
            break;
        }
    }

    actor.stopped().await
}
