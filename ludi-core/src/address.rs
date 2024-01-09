use std::future::Future;

use futures_util::Sink;

use crate::{Envelope, Message, MessageError, Wrap};

/// An address of a mailbox.
///
/// An address is used to send messages to a mailbox, which can be dispatched to an actor.
pub trait Address:
    Sink<Envelope<Self::Message, <Self::Message as Message>::Return>, Error = MessageError>
    + Send
    + 'static
{
    /// The type of message that can be sent to this address.
    type Message: Message;

    /// Sends a message and awaits a response.
    fn send_await<T>(&self, msg: T) -> impl Future<Output = Result<T::Return, MessageError>> + Send
    where
        Self::Message: Wrap<T>,
        T: Message;

    /// Sends a message.
    fn send<T>(&self, msg: T) -> impl Future<Output = Result<(), MessageError>> + Send
    where
        Self::Message: Wrap<T>,
        T: Message;
}

#[cfg(feature = "futures-mailbox")]
pub(crate) mod futures_address {
    use super::*;
    use futures_channel::mpsc;
    use futures_util::SinkExt;
    use std::{pin::Pin, task::Poll};

    /// A MPSC address implemented using channels from the [`futures_channel`](https://crates.io/crates/futures_channel) crate.
    pub struct FuturesAddress<T: Message> {
        pub(crate) send: mpsc::Sender<Envelope<T, T::Return>>,
    }

    impl<T: Message> Clone for FuturesAddress<T> {
        fn clone(&self) -> Self {
            Self {
                send: self.send.clone(),
            }
        }
    }

    impl<T> Sink<Envelope<T, T::Return>> for FuturesAddress<T>
    where
        T: Message,
    {
        type Error = MessageError;

        fn poll_ready(
            mut self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Pin::new(&mut self.send)
                .poll_ready(cx)
                .map_err(|_| MessageError::Closed)
        }

        fn start_send(
            mut self: Pin<&mut Self>,
            item: Envelope<T, T::Return>,
        ) -> Result<(), Self::Error> {
            Pin::new(&mut self.send)
                .start_send(item)
                .map_err(|_| MessageError::Closed)
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Pin::new(&mut self.send)
                .poll_flush(cx)
                .map_err(|_| MessageError::Closed)
        }

        fn poll_close(
            mut self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Pin::new(&mut self.send)
                .poll_close(cx)
                .map_err(|_| MessageError::Closed)
        }
    }

    impl<T> Address for FuturesAddress<T>
    where
        T: Message,
    {
        type Message = T;

        async fn send_await<U>(&self, msg: U) -> Result<U::Return, MessageError>
        where
            Self::Message: Wrap<U>,
            U: Message,
        {
            let (env, ret) = Envelope::new_returning(msg.into());

            self.send
                .clone()
                .send(env)
                .await
                .map_err(|_| MessageError::Closed)?;

            let ret = ret.await.map_err(|_| MessageError::Interrupted)?;

            Self::Message::unwrap_return(ret)
        }

        async fn send<U>(&self, msg: U) -> Result<(), MessageError>
        where
            Self::Message: Wrap<U>,
            U: Message,
        {
            self.send
                .clone()
                .send(Envelope::new(msg.into()))
                .await
                .map_err(|_| MessageError::Closed)
        }
    }
}
