use gpio_cdev::*;
use log::*;
use serde_derive::{Deserialize, Serialize};
use std::{
    thread,
    time::{Duration, Instant},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct BlinkArguments {
    pub duration: Duration,
    pub period: Duration,
}

pub fn blink(chip: &String, line: &u32, args: &BlinkArguments) -> errors::Result<()> {
    debug!("blinking {:?}...", &args);

    let mut chip = Chip::new(chip)?;

    let handle = chip
        .get_line(*line)?
        .request(LineRequestFlags::OUTPUT, 1, "readinput")?;

    let start_time = Instant::now();
    while start_time.elapsed() < args.duration {
        thread::sleep(args.period);
        handle.set_value(0)?;
        thread::sleep(args.period);
        handle.set_value(1)?;
    }

    Ok(())
}
