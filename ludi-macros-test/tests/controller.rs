#![allow(dead_code)]

#[derive(ludi::Controller)]
pub struct Foo;

impl ::ludi::Actor for Foo {
    type Stop = ();
    type Error = ();

    async fn stopped(&mut self) -> Result<Self::Stop, Self::Error> {
        Ok(())
    }
}

#[derive(ludi::Controller)]
pub struct FooGenerics<T>(T)
where
    T: Send;

impl<T: Send> ::ludi::Actor for FooGenerics<T> {
    type Stop = ();
    type Error = ();

    async fn stopped(&mut self) -> Result<Self::Stop, Self::Error> {
        Ok(())
    }
}
