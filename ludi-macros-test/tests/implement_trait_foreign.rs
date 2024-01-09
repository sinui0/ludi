#![allow(dead_code)]

use ludi_macros_test::*;

#[derive(Default, ludi::Controller)]
pub struct Foo;

impl ludi::Actor for Foo {
    type Stop = ();
    type Error = ();

    async fn stopped(&mut self) -> Result<Self::Stop, Self::Error> {
        Ok(())
    }
}

mod foreign_trait {
    #[async_trait::async_trait]
    pub trait Bar {
        async fn bar(&self) -> String;
    }
}

#[ludi::implement(ctrl, msg(foreign, wrap))]
#[async_trait::async_trait]
impl foreign_trait::Bar for Foo {
    async fn bar(&self) -> String {
        unimplemented!()
    }
}

#[test]
fn test_implement_trait() {
    assert_message::<FooBarMsg, FooBarMsgReturn>();
    assert_message::<BarMsgBar, String>();
    assert_wrap::<FooBarMsg, BarMsgBar>();
    assert_handler::<Foo, BarMsgBar>();
}
