#![deny(unsafe_code)]

pub mod envelope;
pub mod mailbox;

use std::marker::PhantomData;

use envelope::ActorEnvelope;
use futures::{Future, Stream, StreamExt};

pub use envelope::Envelope;

pub trait Message<A: Actor>: Send + Sized + 'static {
    type Return: Send + 'static;

    fn handle<M: Mailbox<A>, R: FnOnce(Self::Return) + Send>(
        self,
        actor: &mut A,
        ctx: &mut Context<A, M>,
        ret: R,
    ) -> impl Future<Output = ()> + Send;
}

pub trait Actor: Send + Sized + 'static {
    type Message: Message<Self>;
    type Stop;

    fn started(
        &mut self,
        _ctx: &mut Context<'_, Self, impl Mailbox<Self>>,
    ) -> Result<(), Self::Stop> {
        Ok(())
    }

    fn stopped(&mut self) -> impl Future<Output = Self::Stop> + Send;

    fn run(&mut self, mut mailbox: impl Mailbox<Self>) -> impl Future<Output = Self::Stop> + Send {
        async move {
            let mut ctx = Context::new(&mut mailbox);
            if let Err(stop) = self.started(&mut ctx) {
                return stop;
            }

            while let Some(msg) = mailbox.next().await {
                let mut ctx = Context::new(&mut mailbox);
                msg.handle(self, &mut ctx).await;

                if ctx.stopped() {
                    break;
                }
            }

            self.stopped().await
        }
    }
}

pub trait Handler<T>: Actor {
    type Return: Send + 'static;

    fn handle<M: Mailbox<Self>>(
        &mut self,
        msg: T,
        ctx: &mut Context<Self, M>,
    ) -> impl Future<Output = Self::Return> + Send;

    fn after<M: Mailbox<Self>>(
        &mut self,
        _ctx: &mut Context<Self, M>,
    ) -> impl Future<Output = ()> + Send {
        async {}
    }
}

pub trait Mailbox<A: Actor>: Stream<Item = ActorEnvelope<A>> + Send + Unpin + 'static {
    type Address: Address<A>;

    fn address(&self) -> &Self::Address;
}

pub trait Address<A: Actor>: Clone + Send + 'static {
    fn send<M>(&self, msg: M) -> impl Future<Output = <A as Handler<M>>::Return> + Send
    where
        A: Handler<M>,
        <A::Message as Message<A>>::Return: Into<<A as Handler<M>>::Return>,
        M: Into<A::Message> + Send;
}

pub struct Context<'a, A: Actor, M: Mailbox<A>> {
    running: bool,
    mailbox: &'a mut M,
    _pd: PhantomData<A>,
}

impl<'a, A: Actor, M: Mailbox<A>> Context<'a, A, M> {
    pub fn new(mailbox: &'a mut M) -> Self {
        Self {
            running: true,
            mailbox,
            _pd: PhantomData,
        }
    }

    pub fn mailbox(&mut self) -> &mut M {
        self.mailbox
    }

    pub fn stop(&mut self) {
        self.running = false;
    }

    pub fn stopped(&self) -> bool {
        !self.running
    }
}
