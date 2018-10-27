use log::*;
// use sysfs_gpio::{Direction, Edge, Pin};
use rppal::gpio::{Gpio, Level, Mode, Trigger};

// pub fn interrupt<C>(pin: &u64, mut callback: C) -> sysfs_gpio::Result<()>
// where
//     C: Fn() + Send + 'static,
// {
//     let input = Pin::new(*pin);
//     input.export()?;
//     input.set_direction(Direction::In)?;
//     input.set_edge(Edge::RisingEdge)?;

//     // callback(&input.get_poller()?);
//     let ref mut poller = &input.get_poller()?;
//     loop {
//         if let Some(1) = poller.poll(1000)? {
//             debug!("pressed...");
//             callback();
//         }
//     }
// }

pub fn interrupt<C>(pin: &u8, callback: C)
where
    C: FnMut(Level) + Send + 'static,
{
    let mut gpio = Gpio::new().unwrap();
    gpio.set_mode(*pin, Mode::Output);
    gpio.set_async_interrupt(*pin, Trigger::RisingEdge, callback);
}
