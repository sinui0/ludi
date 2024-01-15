use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use futures_util::StreamExt;

use crate::{
    channel::{new_channel, new_unbounded_channel, Receiver},
    Address, Envelope, Message,
};

/// Returns a new mailbox and address.
pub fn mailbox<T: Message>(capacity: usize) -> (Mailbox<T>, Address<T>) {
    let (sender, recv) = new_channel(capacity);

    (Mailbox { recv }, Address::new(sender))
}

/// Returns a new unbounded mailbox and address.
pub fn unbounded_mailbox<T: Message>() -> (Mailbox<T>, Address<T>) {
    let (sender, recv) = new_unbounded_channel();

    (Mailbox { recv }, Address::new(sender))
}

/// A mailbox.
pub struct Mailbox<T: Message> {
    recv: Receiver<T>,
}

impl<T: Message> Stream for Mailbox<T> {
    type Item = Envelope<T>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.recv.poll_next_unpin(cx)
    }
}

/// An extension trait which converts a stream of messages into a mailbox.
pub trait IntoMailbox: Sized {
    /// Convert self into a mailbox.
    fn into_mailbox(self) -> IntoMail<Self>;
}

impl<T> IntoMailbox for T
where
    T: Stream + Unpin,
    T::Item: Message,
{
    fn into_mailbox(self) -> IntoMail<Self> {
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
    T: Stream + Unpin,
    T::Item: Message,
{
    type Item = Envelope<T::Item>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Stream::poll_next(Pin::new(&mut self.get_mut().0), cx).map(|m| m.map(|m| Envelope::new(m)))
    }
}
