#![allow(dead_code)]

use ludi_macros_test::*;

#[ludi::interface(msg(wrap))]
#[async_trait::async_trait]
pub trait Bar {
    async fn bar(&self) -> String;
}

#[test]
fn test_implement_trait() {
    assert_message::<BarMsg, BarMsgReturn>();
    assert_message::<BarMsgBar, String>();
    assert_wrap::<BarMsg, BarMsgBar>();
}
