pub fn assert_message<T, U>()
where
    T: ludi::Message<Return = U>,
{
}

pub fn assert_wrap<T, U>()
where
    T: ludi::Wrap<U>,
    U: ludi::Message,
{
}

pub fn assert_handler<T, U>()
where
    T: ludi::Handler<U>,
    U: ludi::Message,
{
}
