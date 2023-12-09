use ludi_core::{mailbox::FuturesMailbox, *};

struct PingActor;

impl Actor for PingActor {
    type Message = PingMessage;

    type Stop = ();

    async fn stopped(&mut self) -> Self::Stop {}
}

impl Handler<Ping> for PingActor {
    type Return = String;

    async fn handle<M: Mailbox<Self>>(
        &mut self,
        _msg: Ping,
        _ctx: &mut Context<'_, Self, M>,
    ) -> Self::Return {
        println!("ping");
        "pong".to_string()
    }
}

impl Handler<Pong> for PingActor {
    type Return = ();

    async fn handle<M: Mailbox<Self>>(
        &mut self,
        _msg: Pong,
        _ctx: &mut Context<'_, Self, M>,
    ) -> Self::Return {
        println!("pong");
    }

    async fn after<M: Mailbox<Self>>(&mut self, _ctx: &mut Context<'_, Self, M>) {
        println!("sent pong");
    }
}

enum PingMessage {
    Ping(Ping),
    Pong(Pong),
}

enum PingReturn {
    Ping(String),
    Pong(()),
}

impl Into<()> for PingReturn {
    fn into(self) -> () {
        match self {
            PingReturn::Pong(()) => (),
            _ => unreachable!("handler returned unexpected type, this indicates the `Message` implementation is incorrect"),
        }
    }
}

impl Into<String> for PingReturn {
    fn into(self) -> String {
        match self {
            PingReturn::Ping(s) => s,
            _ => unreachable!("handler returned unexpected type, this indicates the `Message` implementation is incorrect"),
        }
    }
}

impl Message<PingActor> for PingMessage {
    type Return = PingReturn;

    async fn handle<M: Mailbox<PingActor>, R: FnOnce(PingReturn)>(
        self,
        actor: &mut PingActor,
        ctx: &mut Context<'_, PingActor, M>,
        ret: R,
    ) {
        match self {
            PingMessage::Ping(ping) => {
                let value = PingReturn::Ping(Handler::<Ping>::handle(actor, ping, ctx).await);
                ret(value);
                Handler::<Ping>::after(actor, ctx).await;
            }
            PingMessage::Pong(pong) => {
                let value = PingReturn::Pong(Handler::<Pong>::handle(actor, pong, ctx).await);
                ret(value);
                Handler::<Pong>::after(actor, ctx).await;
            }
        };
    }
}

struct Ping;

impl From<Ping> for PingMessage {
    fn from(value: Ping) -> Self {
        PingMessage::Ping(value)
    }
}

struct Pong;

impl From<Pong> for PingMessage {
    fn from(value: Pong) -> Self {
        PingMessage::Pong(value)
    }
}

#[tokio::test]
async fn test_api() {
    let mailbox = FuturesMailbox::new();
    let addr = mailbox.address().clone();
    let mut actor = PingActor;

    tokio::spawn(async move { actor.run(mailbox).await });

    addr.send(Pong).await;
}
