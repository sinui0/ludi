//! Futures for sending messages and waiting for responses.

use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::{ready, FusedFuture, Future};
use futures_util::FutureExt;

use crate::{
    channel::{new_response, ChannelError, OneshotReceiver, ResponseSender, Sender},
    Envelope, Error, Message,
};

/// A [`MessageFuture`] mode which will wait for a response.
pub struct Wait;
/// A [`MessageFuture`] mode which will return a [`ResponseFuture`].
pub struct Detach;

/// A future which sends a message and optionally waits for a response depending on the mode.
///
/// # Modes
///
/// * [`Wait`] - Waits for a response.
/// * [`Detach`] - Returns a [`ResponseFuture`] which can be used to wait for the response.
#[must_use = "futures do nothing unless polled"]
pub struct MessageFuture<T: Message, M> {
    queue: QueueFuture<T>,
    response: Option<ResponseFuture<T>>,
    _mode: PhantomData<M>,
}

impl<T: Message> MessageFuture<T, Wait> {
    pub(crate) fn new(queue: QueueFuture<T>, response: ResponseFuture<T>) -> Self {
        Self {
            queue,
            response: Some(response),
            _mode: PhantomData,
        }
    }

    /// Returns a new [`MessageFuture`] which will instead resolve when the message is sent and
    /// return a [`ResponseFuture`] which can be used to wait for the response.
    pub fn detach(self) -> MessageFuture<T, Detach> {
        MessageFuture {
            queue: self.queue,
            response: self.response,
            _mode: PhantomData,
        }
    }
}

impl<T: Message> Future for MessageFuture<T, Wait> {
    type Output = Result<T::Return, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if !this.queue.is_terminated() {
            ready!(this.queue.poll_unpin(cx))?;
        }

        let Some(response) = this.response.as_mut() else {
            panic!("future is not polled after completion")
        };

        response.poll_unpin(cx)
    }
}

impl<T: Message> Future for MessageFuture<T, Detach> {
    type Output = Result<ResponseFuture<T>, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if !this.queue.is_terminated() {
            ready!(this.queue.poll_unpin(cx))?;
        }

        Poll::Ready(Ok(this
            .response
            .take()
            .expect("future is not polled after completion")))
    }
}

impl<T: Message> FusedFuture for MessageFuture<T, Wait> {
    fn is_terminated(&self) -> bool {
        self.queue.is_terminated()
            && self
                .response
                .as_ref()
                .map(|r| r.is_terminated())
                .unwrap_or(true)
    }
}

/// A future which resolves when a message is successfully queued.
#[must_use = "futures do nothing unless polled"]
pub struct QueueFuture<T: Message> {
    sender: Sender<T>,
    msg: Option<Envelope<T>>,
}

impl<T: Message> QueueFuture<T> {
    pub(crate) fn new(sender: Sender<T>, msg: Envelope<T>) -> Self {
        Self {
            sender,
            msg: Some(msg),
        }
    }
}

impl<T: Message> Future for QueueFuture<T> {
    type Output = Result<(), Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        if this.msg.is_some() {
            ready!(this.sender.poll_ready(cx).map_err(|_| Error::Disconnected))?;
            if let Err(err) = this.sender.try_send(this.msg.take().unwrap()) {
                match err {
                    ChannelError::Disconnected => return Poll::Ready(Err(Error::Disconnected)),
                    ChannelError::Full(msg) => {
                        this.msg = Some(msg);
                        return Poll::Pending;
                    }
                }
            }
        }

        Poll::Ready(Ok(()))
    }
}

impl<T: Message> FusedFuture for QueueFuture<T> {
    fn is_terminated(&self) -> bool {
        self.msg.is_none()
    }
}

/// A future which returns the response to a message.
#[must_use = "futures do nothing unless polled"]
pub struct ResponseFuture<T: Message>(pub(crate) OneshotReceiver<T::Return>);

impl<T: Message> ResponseFuture<T> {
    /// Returns a new [`ResponseSender`] and [`ResponseFuture`].
    pub fn new() -> (ResponseSender<T>, Self) {
        new_response()
    }
}

impl<T: Message> Future for ResponseFuture<T> {
    type Output = Result<T::Return, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().0)
            .poll(cx)
            .map_err(|_| Error::Interrupted)
    }
}

impl<T: Message> FusedFuture for ResponseFuture<T> {
    fn is_terminated(&self) -> bool {
        self.0.is_terminated()
    }
}
