#![allow(dead_code)]

use ludi_macros_test::*;

#[derive(Default, ludi::Controller)]
pub struct CounterBoi {
    count: u32,
}

impl ludi::Actor for CounterBoi {
    type Stop = ();
    type Error = ();

    async fn stopped(&mut self) -> Result<Self::Stop, Self::Error> {
        Ok(())
    }
}

#[ludi::implement]
#[ctrl]
#[msg(wrap)]
impl CounterBoi {
    pub async fn increment(&mut self) -> u32 {
        self.count += 1;
        self.count
    }
}

#[test]
fn test() {
    assert_message::<CounterBoiMsg, CounterBoiMsgReturn>();
    assert_message::<CounterBoiMsgIncrement, u32>();
    assert_wrap::<CounterBoiMsg, CounterBoiMsgIncrement>();
    assert_handler::<CounterBoi, CounterBoiMsgIncrement>();
}
