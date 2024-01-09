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

#[ludi::interface(msg(wrap))]
#[async_trait::async_trait]
pub trait Bar {
    async fn bar(&self) -> String;
}

#[ludi::implement(ctrl)]
#[async_trait::async_trait]
impl Bar for Foo {
    async fn bar(&self) -> String {
        unimplemented!()
    }
}

#[test]
fn test_implement_trait() {
    assert_message::<BarMsg, BarMsgReturn>();
    assert_message::<BarMsgBar, String>();
    assert_wrap::<BarMsg, BarMsgBar>();
    assert_handler::<Foo, BarMsgBar>();
}
