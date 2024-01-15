use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::Stream;
use futures_util::Sink;

use crate::{futures::ResponseFuture, Envelope, Error, Message};

// TODO: Support other channel implementations using conditional compilation.
pub(crate) type OneshotSender<T> = futures_channel::oneshot::Sender<T>;
pub(crate) type OneshotReceiver<T> = futures_channel::oneshot::Receiver<T>;
pub(crate) type BoundedSender<T> = futures_channel::mpsc::Sender<Envelope<T>>;
pub(crate) type BoundedReceiver<T> = futures_channel::mpsc::Receiver<Envelope<T>>;
pub(crate) type UnboundedSender<T> = futures_channel::mpsc::UnboundedSender<Envelope<T>>;
pub(crate) type UnboundedReceiver<T> = futures_channel::mpsc::UnboundedReceiver<Envelope<T>>;

pub(crate) fn new_response<T: Message>() -> (ResponseSender<T>, ResponseFuture<T>) {
    let (sender, receiver) = futures_channel::oneshot::channel();

    (ResponseSender(sender), ResponseFuture(receiver))
}

/// A channel for sending a response to a message.
pub struct ResponseSender<T: Message>(OneshotSender<T::Return>);

impl<T> std::fmt::Debug for ResponseSender<T>
where
    T: Message + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ResponseSender").field(&self.0).finish()
    }
}

impl<T: Message> ResponseSender<T> {
    /// Sends the response.
    pub fn send(self, msg: T::Return) {
        // Ignore the error if the receiver has been dropped.
        _ = self.0.send(msg);
    }
}

pub(crate) fn new_channel<T: Message>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = futures_channel::mpsc::channel(capacity);

    (Sender::Bounded(sender), Receiver::Bounded(receiver))
}

pub(crate) fn new_unbounded_channel<T: Message>() -> (Sender<T>, Receiver<T>) {
    let (sender, receiver) = futures_channel::mpsc::unbounded();

    (Sender::Unbounded(sender), Receiver::Unbounded(receiver))
}

#[derive(Debug)]
pub(crate) enum Sender<T: Message> {
    Bounded(BoundedSender<T>),
    Unbounded(UnboundedSender<T>),
}

impl<T: Message> Clone for Sender<T> {
    fn clone(&self) -> Self {
        match self {
            Self::Bounded(sender) => Self::Bounded(sender.clone()),
            Self::Unbounded(sender) => Self::Unbounded(sender.clone()),
        }
    }
}

pub(crate) enum ChannelError<T> {
    /// The channel is closed.
    Disconnected,
    /// The channel is full.
    Full(T),
}

pub(crate) struct Disconnected;

impl<T: Message> Sender<T> {
    pub(crate) fn close(&mut self) {
        match self {
            Self::Bounded(sender) => sender.close_channel(),
            Self::Unbounded(sender) => sender.close_channel(),
        }
    }

    pub(crate) fn is_closed(&self) -> bool {
        match self {
            Self::Bounded(sender) => sender.is_closed(),
            Self::Unbounded(sender) => sender.is_closed(),
        }
    }

    pub(crate) fn poll_ready(&mut self, ctx: &mut Context<'_>) -> Poll<Result<(), Disconnected>> {
        match self {
            Self::Bounded(sender) => sender.poll_ready(ctx).map_err(|_| Disconnected),
            Self::Unbounded(sender) => sender.poll_ready(ctx).map_err(|_| Disconnected),
        }
    }

    pub(crate) fn try_send(
        &mut self,
        envelope: Envelope<T>,
    ) -> Result<(), ChannelError<Envelope<T>>> {
        match self {
            Self::Bounded(sender) => sender.try_send(envelope).map_err(|e| {
                if e.is_full() {
                    ChannelError::Full(e.into_inner())
                } else {
                    ChannelError::Disconnected
                }
            }),
            Self::Unbounded(sender) => sender.unbounded_send(envelope).map_err(|e| {
                if e.is_full() {
                    unreachable!("Unbounded channels cannot be full")
                } else {
                    ChannelError::Disconnected
                }
            }),
        }
    }
}

impl<T: Message> Sink<Envelope<T>> for Sender<T> {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            Sender::Bounded(sender) => sender.poll_ready(cx).map_err(|_| Error::Disconnected),
            Sender::Unbounded(sender) => sender.poll_ready(cx).map_err(|_| Error::Disconnected),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: Envelope<T>) -> Result<(), Self::Error> {
        match self.get_mut() {
            Sender::Bounded(sender) => sender.start_send(item).map_err(|_| Error::Disconnected),
            Sender::Unbounded(sender) => sender.start_send(item).map_err(|_| Error::Disconnected),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            Sender::Bounded(sender) => Pin::new(sender)
                .poll_flush(cx)
                .map_err(|_| Error::Disconnected),
            Sender::Unbounded(sender) => Pin::new(sender)
                .poll_flush(cx)
                .map_err(|_| Error::Disconnected),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            Sender::Bounded(sender) => Pin::new(sender)
                .poll_close(cx)
                .map_err(|_| Error::Disconnected),
            Sender::Unbounded(sender) => Pin::new(sender)
                .poll_close(cx)
                .map_err(|_| Error::Disconnected),
        }
    }
}

#[derive(Debug)]
pub(crate) enum Receiver<T: Message> {
    Bounded(BoundedReceiver<T>),
    Unbounded(UnboundedReceiver<T>),
}

impl<T: Message> Stream for Receiver<T> {
    type Item = Envelope<T>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.get_mut() {
            Receiver::Bounded(receiver) => Pin::new(receiver).poll_next(cx),
            Receiver::Unbounded(receiver) => Pin::new(receiver).poll_next(cx),
        }
    }
}
