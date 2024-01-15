use std::{
    pin::Pin,
    task::{Context, Poll},
};

use futures_util::Sink;

use crate::{
    channel::Sender,
    futures::{MessageFuture, QueueFuture, Wait},
    Envelope, Error, Message, Wrap,
};

/// An address which can be used to send messages to a mailbox.
#[derive(Debug)]
pub struct Address<T: Message> {
    sender: Sender<T>,
}

impl<T: Message> Address<T> {
    pub(crate) fn new(sender: Sender<T>) -> Self {
        Self { sender }
    }

    /// Closes the mailbox with this address.
    pub fn close(&mut self) {
        self.sender.close();
    }

    /// Returns whether the mailbox is connected.
    pub fn is_connected(&self) -> bool {
        self.sender.is_closed()
    }

    /// Sends a message and waits for a response.
    pub async fn send<U>(&self, msg: U) -> Result<U::Return, Error>
    where
        T: Wrap<U>,
        U: Message,
    {
        T::unwrap_return(self.wait(msg.into()).await?)
    }

    /// Returns a future which resolves immediately when a message is queued.
    pub fn queue(&self, msg: T) -> QueueFuture<T> {
        QueueFuture::new(self.sender.clone(), Envelope::new(msg))
    }

    /// Returns a future which will send a message and wait for a response.
    pub fn wait(&self, msg: T) -> MessageFuture<T, Wait> {
        let (envelope, response) = Envelope::new_with_response(msg);
        MessageFuture::new(QueueFuture::new(self.sender.clone(), envelope), response)
    }
}

impl<T: Message> Clone for Address<T> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<T: Message> Sink<Envelope<T>> for Address<T> {
    type Error = Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().sender)
            .poll_ready(cx)
            .map_err(|_| Error::Disconnected)
    }

    fn start_send(self: Pin<&mut Self>, item: Envelope<T>) -> Result<(), Self::Error> {
        Pin::new(&mut self.get_mut().sender)
            .start_send(item)
            .map_err(|_| Error::Disconnected)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().sender)
            .poll_flush(cx)
            .map_err(|_| Error::Disconnected)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Pin::new(&mut self.get_mut().sender)
            .poll_close(cx)
            .map_err(|_| Error::Disconnected)
    }
}
