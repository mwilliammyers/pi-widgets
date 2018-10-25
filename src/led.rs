use log::*;
use serde_derive::{Deserialize, Serialize};
use std::{thread, time::Duration};
use sysfs_gpio::{Direction, Pin};

#[derive(Serialize, Deserialize, Debug)]
pub struct BlinkArguments {
    pub duration_ms: u64,
    pub period_ms: u64,
}

pub fn blink(pin: &u64, args: &BlinkArguments) -> sysfs_gpio::Result<()> {
    debug!("Blinking LED...");

    let led = Pin::new(*pin);
    led.with_exported(|| {
        led.set_direction(Direction::Low)?;

        let iterations = args.duration_ms / args.period_ms / 2;
        for _ in 0..iterations {
            led.set_value(0)?;
            thread::sleep(Duration::from_millis(args.period_ms));
            led.set_value(1)?;
            thread::sleep(Duration::from_millis(args.period_ms));
        }
        led.set_value(0)?;

        Ok(())
    })
}
