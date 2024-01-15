use crate::{channel::ResponseSender, futures::ResponseFuture, Actor, Context, Dispatch, Message};

pub(crate) enum EnvelopeInner<T: Message> {
    /// A message which does not expect a response.
    NoResponse(T),
    /// A message which expects a response.
    WantsResponse(T, ResponseSender<T>),
}

/// An envelope containing a message and optionally a channel which can be
/// used to return a response back to the sender.
pub struct Envelope<T: Message>(EnvelopeInner<T>);

impl<T: Message> Envelope<T> {
    /// Create a new envelope.
    pub fn new(msg: T) -> Self {
        Self(EnvelopeInner::NoResponse(msg))
    }

    /// Create a new envelope with a channel which can be used to return
    /// a response to the sender.
    pub fn new_with_response(msg: T) -> (Self, ResponseFuture<T>) {
        let (send, recv) = ResponseFuture::new();
        (Self(EnvelopeInner::WantsResponse(msg, send)), recv)
    }

    /// Returns `true` if the envelope has a channel which will receive a response.
    pub fn wants_response(&self) -> bool {
        match &self.0 {
            EnvelopeInner::NoResponse(_) => false,
            EnvelopeInner::WantsResponse(_, _) => true,
        }
    }

    /// Dispatches the message and return channel to the actor for handling.
    ///
    /// # Arguments
    ///
    /// * `actor` - The actor which will handle the message.
    /// * `ctx` - The context of the actor.
    pub async fn dispatch<A>(self, actor: &mut A, ctx: &mut Context<A>)
    where
        A: Actor,
        T: Dispatch<A>,
    {
        match self.0 {
            EnvelopeInner::NoResponse(msg) => {
                msg.dispatch(actor, ctx, move |_| {}).await;
            }
            EnvelopeInner::WantsResponse(msg, sender) => {
                msg.dispatch(actor, ctx, move |ret| {
                    let _ = sender.send(ret);
                })
                .await;
            }
        }
    }
}
