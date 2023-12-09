use futures::{channel::mpsc, SinkExt, Stream};

use crate::{Actor, Address, Envelope, Handler, Mailbox, Message};

pub struct FuturesMailbox<A: Actor> {
    addr: FuturesAddress<A>,
    recv: mpsc::Receiver<Envelope<A::Message, <A::Message as Message<A>>::Return, A>>,
}

impl<A: Actor> FuturesMailbox<A> {
    pub fn new() -> Self {
        let (send, recv) = mpsc::channel(100);

        Self {
            addr: FuturesAddress { send },
            recv,
        }
    }
}

impl<A: Actor> Mailbox<A> for FuturesMailbox<A> {
    type Address = FuturesAddress<A>;

    fn address(&self) -> &Self::Address {
        &self.addr
    }
}

impl<A: Actor> Stream for FuturesMailbox<A> {
    type Item = Envelope<A::Message, <A::Message as Message<A>>::Return, A>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        std::pin::Pin::new(&mut self.recv).poll_next(cx)
    }
}

pub struct FuturesAddress<A: Actor> {
    send: mpsc::Sender<Envelope<A::Message, <A::Message as Message<A>>::Return, A>>,
}

impl<A: Actor> Clone for FuturesAddress<A> {
    fn clone(&self) -> Self {
        Self {
            send: self.send.clone(),
        }
    }
}

impl<A: Actor> Address<A> for FuturesAddress<A> {
    async fn send<M>(&self, msg: M) -> <A as Handler<M>>::Return
    where
        A: Handler<M>,
        <A::Message as Message<A>>::Return: Into<<A as Handler<M>>::Return>,
        M: Into<A::Message> + Send,
    {
        let msg: A::Message = msg.into();

        let (env, ret) = Envelope::new_returning(msg.into());

        let mut send = self.send.clone();

        send.send(env).await.unwrap();

        let ret: <A::Message as Message<A>>::Return = ret.await.unwrap();

        ret.into()
    }
}
