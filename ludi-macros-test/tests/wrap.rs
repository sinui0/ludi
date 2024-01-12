#![allow(dead_code)]

use ludi_macros_test::*;

#[derive(ludi::Message)]
#[ludi(return_ty = usize)]
struct Foo;

#[derive(ludi::Message)]
struct Bar;

#[derive(ludi::Wrap)]
#[ludi(return_attrs(derive(Clone)))]
enum Baz {
    Foo(Foo),
    Bar(Bar),
}

#[test]
fn test_wrap() {
    assert_message::<Foo, usize>();
    assert_message::<Bar, ()>();
    assert_message::<Baz, BazReturn>();
    assert_wrap::<Baz, Foo>();
    assert_wrap::<Baz, Bar>();
    assert_clone::<BazReturn>();
}

fn assert_clone<T: Clone>() {}
