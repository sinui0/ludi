#![allow(dead_code)]

use ludi_macros_test::*;

mod msg {
    #[derive(ludi::Message)]
    #[ludi(return_ty = usize)]
    pub struct Foo;
}

#[derive(ludi::Message)]
#[ludi(return_ty = String)]
struct Bar {
    bar: String,
}

#[derive(ludi::Wrap)]
enum CounterMessage {
    Foo(msg::Foo),
    Bar(Bar),
}

#[test]
fn test() {
    assert_message::<CounterMessage, CounterMessageReturn>();
    assert_wrap::<CounterMessage, msg::Foo>();
    assert_wrap::<CounterMessage, Bar>();
}
