use std::{pin::Pin, task::Poll};

use futures_core::Stream;

use crate::{Envelope, Message};

/// A mailbox.
///
/// A mailbox is an asynchronous stream of messages which can be dispatched to an actor. The counter-part
/// to a mailbox is an [`Address`](crate::Address), which is used to send messages.
pub trait Mailbox:
    Stream<Item = Envelope<Self::Message, <Self::Message as Message>::Return>> + Send + Unpin + 'static
{
    /// The type of message that can be sent to this mailbox.
    type Message: Message;
}

impl<T, U> Mailbox for T
where
    T: Stream<Item = Envelope<U, <U as Message>::Return>> + Send + Unpin + 'static,
    U: Message,
{
    type Message = U;
}

/// An extension trait which converts a stream of messages into a mailbox.
pub trait IntoMailbox {
    /// The type of message that can be sent to the mailbox.
    type Message: Message;
    /// The type of mailbox.
    type IntoMail: Mailbox<Message = Self::Message>;

    /// Convert self into a mailbox.
    fn into_mailbox(self) -> Self::IntoMail;
}

impl<T> IntoMailbox for T
where
    T: Stream + Send + Unpin + 'static,
    T::Item: Message,
{
    type Message = T::Item;
    type IntoMail = IntoMail<T>;

    fn into_mailbox(self) -> Self::IntoMail {
        IntoMail(self)
    }
}

/// Adapter returned from [`IntoMailbox::into_mailbox`].
///
/// Used to convert a stream of messages into a mailbox.
pub struct IntoMail<T>(T);

impl<T> IntoMail<T> {
    /// Returns the inner stream.
    pub fn to_inner(self) -> T {
        self.0
    }

    /// Returns a reference to the inner stream.
    pub fn inner_ref(&self) -> &T {
        &self.0
    }

    /// Returns a mutable reference to the inner stream.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> Stream for IntoMail<T>
where
    T: Stream + Send + Unpin + 'static,
    T::Item: Message,
{
    type Item = Envelope<T::Item, <T::Item as Message>::Return>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Stream::poll_next(Pin::new(&mut self.get_mut().0), cx).map(|m| m.map(|m| Envelope::new(m)))
    }
}

#[cfg(feature = "futures-mailbox")]
pub(crate) mod futures_mailbox {
    use super::*;
    use crate::FuturesAddress;
    use futures_channel::mpsc;

    /// Returns a new mailbox and its' address.
    ///
    /// # Arguments
    ///
    /// * `capacity` - The number of messages that can be buffered in the mailbox.
    pub fn mailbox<T>(capacity: usize) -> (FuturesMailbox<T>, FuturesAddress<T>)
    where
        T: Message,
    {
        FuturesMailbox::new(capacity)
    }

    /// A MPSC mailbox implemented using channels from the [`futures_channel`](https://crates.io/crates/futures_channel) crate.
    pub struct FuturesMailbox<T: Message> {
        recv: mpsc::Receiver<Envelope<T, T::Return>>,
    }

    impl<T: Message> FuturesMailbox<T> {
        /// Create a new mailbox, returning the mailbox and its' address.
        ///
        /// # Arguments
        ///
        /// * `buffer` - The number of messages that can be buffered in the mailbox.
        pub fn new(buffer: usize) -> (Self, FuturesAddress<T>) {
            let (send, recv) = mpsc::channel(buffer);

            (Self { recv }, FuturesAddress { send })
        }
    }

    impl<T: Message> Stream for FuturesMailbox<T> {
        type Item = Envelope<T, T::Return>;

        fn poll_next(
            mut self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> Poll<Option<Self::Item>> {
            Pin::new(&mut self.recv).poll_next(cx)
        }
    }
}
