use std::marker::PhantomData;

use futures::channel::oneshot::{self, Receiver, Sender};

use crate::{Actor, Context, Mailbox, Message};

pub type ActorEnvelope<A> =
    Envelope<<A as Actor>::Message, <<A as Actor>::Message as Message<A>>::Return, A>;

pub struct Envelope<M, R, A> {
    msg: M,
    send: Option<Sender<R>>,
    _pd: PhantomData<A>,
}

impl<M, R, A> Envelope<M, R, A> {
    pub fn new(msg: M) -> Self {
        Self {
            msg,
            send: None,
            _pd: PhantomData,
        }
    }

    pub fn new_returning(msg: M) -> (Self, Receiver<R>) {
        let (send, recv) = oneshot::channel();
        (
            Self {
                msg,
                send: Some(send),
                _pd: PhantomData,
            },
            recv,
        )
    }
}

impl<M, R, A> Envelope<M, R, A>
where
    A: Actor,
    M: Message<A, Return = R> + Send + 'static,
    R: Send + 'static,
{
    pub async fn handle<T: Mailbox<A>>(mut self, actor: &mut A, ctx: &mut Context<'_, A, T>) {
        self.msg
            .handle(actor, ctx, move |ret| {
                if let Some(send) = self.send.take() {
                    let _ = send.send(ret);
                }
            })
            .await;
    }
}
