# ludi

A minimal async actor-like framework written in Rust.

## Overview

ludi is mostly a collection of traits which compose together to resemble an actor framework. It is not a full-featured actor framework for building massively concurrent applications deployed to horizontally scalable clusters, nor is it intended to be. Instead, **ludi focuses on providing a lightweight library specifically for asynchronously managing shared local state via message channels**. The provided abstractions support writing concurrent programs without directly relying on lock-based primitives and all the trickiness that comes with them.

Check out this blog post on [tokio actors](https://ryhl.io/blog/actors-with-tokio/) which serves as a nice introduction to this paradigm.

A pitfall of message-based synchronization, in this author's view, is the boilerplate that comes with it. To address this, ludi comes with (optional) macros which can be used to generate APIs which encapsulate the implementation details of message passing, and instead provide more ergonomic OOP-style interfaces (traits, methods). This approach was inspired by [`spaad`](https://github.com/Restioson/spaad), a crate built on [`xtra`](https://github.com/Restioson/xtra).

## Features

- Small
    - Contains very little implementation code, mostly traits and helpers.
    - The "batteries" that are included are feature gated, eg `futures-mailbox`.
- Ergonomic
    - Generate APIs which resemble lock-based interfaces.
- Safe
    - ludi is `#![deny(unsafe_code)]`
- Flexible
    - Traits are public and low-level, extension crates can support more advanced features.
    - ludi does not have to appear in your own API
- Executor agnostic
    - Not coupled to a runtime such as `tokio`, everything is built on std primitives.
    - Caveat: until RTN, async traits will include `Send` bounds
- Macros to kill boilerplate
    - No magic, the boilerplate can be written by hand instead if that's your preference.

## Example

```rust
// Define an actor struct.
//
// We use the `Controller` macro to generate a controller for the actor.
#[derive(Default, ludi::Controller)]
struct CounterBoi {
    count: usize,
}

impl ludi::Actor for CounterBoi {
    type Stop = ();
    type Error = ();

    async fn stopped(&mut self) -> Result<Self::Stop, Self::Error> {
        Ok(())
    }
}

// Slap `#[interface]` on a trait to generate messages for it.
//
// The `msg(wrap)` attribute generates a wrapper message for the trait, called `CounterMsg`.
#[ludi::interface(msg(wrap))]
trait Counter {
    /// Reset the counter to zero.
    async fn reset(&self);

    /// Return the current value of the counter.
    fn count(&self) -> impl std::future::Future<Output = usize> + Send;

    /// Increment the counter by `increment` and return the new value.
    async fn increment(&self, increment: usize) -> usize;
}

// Implement the trait for the actor as if it were a normal implementation block.
//
// This generates handlers for all the `Counter` trait messages.
//
// We pass in the `ctrl` attribute so that the trait is implemented for the actor's
// controller.
#[ludi::implement(ctrl)]
impl Counter for CounterBoi {
    async fn reset(&self) {
        // `self` is mutable, despite the trait signature.
        self.count = 0;
    }

    async fn count(&self) -> usize {
        self.count
    }

    async fn increment(&self, increment: usize) -> usize {
        self.count += increment;
        self.count
    }
}

#[tokio::main]
async fn main() {
    // Create a mailbox and address for sending `CounterMsg` messages.
    let (mut mailbox, addr) = ludi::mailbox::<CounterMsg>(8);

    // Create a new actor.
    let mut actor = CounterBoi::default();

    // Create a controller for the actor using the address.
    // This controller implements the `Counter` trait.
    let ctrl = CounterBoi::controller(addr);

    // Spawn the actor to run in the background. This works with any executor, not just tokio.
    tokio::spawn(async move { ludi::run(&mut actor, &mut mailbox).await });

    // Tada! No message passing present in the API!
    let count = ctrl.increment(1).await;
    assert_eq!(count, 1);

    ctrl.reset().await;
    assert_eq!(ctrl.count().await, 0);

    let count = ctrl.increment(2).await;
    assert_eq!(count, 2);
}
```

## License

All ludi crates are licensed under either of

- Apache License, Version 2.0
- MIT license

at your option.