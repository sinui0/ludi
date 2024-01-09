use futures_channel::oneshot::{channel, Receiver, Sender};

use crate::{Actor, Context, Dispatch, Message};

/// An envelope containing a message and optionally a channel which can be
/// used to return a value back to the sender.
pub struct Envelope<M, R> {
    msg: M,
    send: Option<Sender<R>>,
}

impl<M, R> Envelope<M, R> {
    /// Create a new envelope.
    pub fn new(msg: M) -> Self {
        Self { msg, send: None }
    }

    /// Create a new envelope with a channel which can be used to return
    /// a response to the sender.
    pub fn new_returning(msg: M) -> (Self, Receiver<R>) {
        let (send, recv) = channel();
        (
            Self {
                msg,
                send: Some(send),
            },
            recv,
        )
    }
}

impl<M, R> Envelope<M, R>
where
    M: Message<Return = R>,
    R: Send,
{
    /// Dispatches the message and return channel to the actor for handling.
    ///
    /// # Arguments
    ///
    /// * `actor` - The actor which will handle the message.
    /// * `ctx` - The context of the actor.
    pub async fn dispatch<A>(mut self, actor: &mut A, ctx: &mut Context<A>)
    where
        A: Actor,
        M: Dispatch<A>,
    {
        self.msg
            .dispatch(actor, ctx, move |ret| {
                if let Some(send) = self.send.take() {
                    let _ = send.send(ret);
                }
            })
            .await
    }
}
