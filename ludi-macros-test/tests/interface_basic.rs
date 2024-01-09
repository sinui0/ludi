#![allow(dead_code, unreachable_code)]

use ludi_macros_test::*;

#[ludi::interface(msg(wrap))]
pub trait Foo {
    #[msg(name = "FooMethod")]
    fn foo(&self, msg: String) -> impl std::future::Future<Output = String>;

    #[allow(async_fn_in_trait)]
    async fn bar(&self);
}

#[test]
fn test() {
    assert_message::<FooMsg, FooMsgReturn>();
    assert_message::<FooMethod, String>();
    assert_message::<FooMsgBar, ()>();
    assert_wrap::<FooMsg, FooMethod>();
    assert_wrap::<FooMsg, FooMsgBar>();
}
