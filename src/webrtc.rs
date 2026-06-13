use std::time::Instant;

use str0m::Rtc;

pub fn new_rtc() -> Rtc {
    Rtc::new(Instant::now())
}