#![allow(dead_code)]

use ludi_macros_test::*;

#[derive(ludi::Message)]
#[ludi(return_ty = usize)]
struct Foo {
    foo: usize,
}

#[test]
fn test() {
    assert_message::<Foo, usize>();
}
