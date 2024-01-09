#![allow(dead_code)]

use ludi_macros_test::*;
use std::future::Future;

#[derive(Default)]
pub struct Foo;

impl ludi::Actor for Foo {
    type Stop = ();
    type Error = ();

    async fn stopped(&mut self) -> Result<Self::Stop, Self::Error> {
        Ok(())
    }
}

#[ludi::interface(msg(wrap))]
trait Bar {
    fn foo(&mut self) -> impl Future<Output = u32>;

    async fn bar(&self) -> String;

    fn baz(&self) -> impl Future<Output = ()> + Send;
}

#[ludi::implement]
impl Bar for Foo {
    #[msg(skip_handler)]
    async fn foo(&mut self) -> u32 {
        unimplemented!()
    }

    async fn bar(&self) -> String {
        unimplemented!()
    }

    async fn baz(&self) {}
}

impl ludi::Handler<BarMsgFoo> for Foo {
    async fn handle(&mut self, _msg: BarMsgFoo, _ctx: &mut ludi::prelude::Context<Self>) -> u32 {
        todo!()
    }
}

#[test]
fn test_implement_trait() {
    assert_message::<BarMsg, BarMsgReturn>();
    assert_message::<BarMsgFoo, u32>();
    assert_message::<BarMsgBar, String>();
    assert_message::<BarMsgBaz, ()>();
    assert_wrap::<BarMsg, BarMsgFoo>();
    assert_wrap::<BarMsg, BarMsgBar>();
    assert_wrap::<BarMsg, BarMsgBaz>();
    assert_handler::<Foo, BarMsgFoo>();
    assert_handler::<Foo, BarMsgBar>();
    assert_handler::<Foo, BarMsgBaz>();
}
