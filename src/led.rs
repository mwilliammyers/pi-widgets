use log::*;
use serde_derive::{Deserialize, Serialize};
use std::{thread, time::Duration};
// use sysfs_gpio::{Direction, Pin};
use rppal::gpio::{Gpio, Level, Mode};

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct BlinkArguments {
    pub duration_ms: u64,
    pub period_ms: u64,
}

pub fn blink(pin: &u8, args: &BlinkArguments) {
    debug!("{:?}", &args);

    let mut gpio = Gpio::new().unwrap();
    gpio.set_mode(*pin, Mode::Output);

    let iterations = args.duration_ms / args.period_ms / 2;
    for _ in 0..iterations {
        gpio.write(*pin, Level::Low);
        thread::sleep(Duration::from_millis(args.period_ms));
        gpio.write(*pin, Level::High);
        thread::sleep(Duration::from_millis(args.period_ms));
    }
    gpio.write(*pin, Level::Low);
}
