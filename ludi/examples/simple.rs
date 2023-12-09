use ludi::{implement, interface, mailbox::FuturesMailbox, prelude::*};

// Slap `#[interface]` on a trait to generate a message types for it.
//
// This also generates a blanket implementation for all addresses of actors which implement
// all the `Handler` impls for the messages.
//
// The original trait is not modified in any way.
#[interface]
trait Counter {
    /// Increment the counter by `increment` and return the new value.
    async fn increment(&self, increment: usize) -> usize;
}

#[derive(Default)]
struct CounterBoi {
    count: usize,
}

impl Actor for CounterBoi {
    type Message = CounterMessage;
    type Stop = ();

    async fn stopped(&mut self) -> Self::Stop {}
}

// Implement a trait for an actor as if it were a normal implementation block.
//
// Code navigation works as expected, at least in VSCode. As in, you can jump to this
// implementation from the trait definition.
#[implement]
impl Counter for CounterBoi {
    async fn increment(&self, increment: usize) -> usize {
        self.count += increment;
        self.count
    }
}

#[tokio::main]
async fn main() {
    let mailbox = FuturesMailbox::new();
    let addr = mailbox.address().clone();
    let mut actor = CounterBoi::default();

    tokio::spawn(async move { actor.run(mailbox).await });

    // Because of the blanket implementation, this actor's address implements
    // the trait directly and can be used as normal.
    //
    // Also because it implements the actual trait documentation, highlighting,
    // etc. works as expected.
    let _count: usize = addr.increment(1).await;

    // And can of course use the address as normal as well.
    let _count: usize = addr.send(counter_msgs::Increment { increment: 1 }).await;

    println!("adding: {}, result: {}", 2, addr.increment(2).await);
    println!("adding: {}, result: {}", 3, addr.increment(3).await);
}
