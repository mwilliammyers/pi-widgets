use log::*;
use sysfs_gpio::{Direction, Edge, Pin};

pub fn interrupt<F: Fn()>(pin: &u64, callback: F) -> sysfs_gpio::Result<()> {
    let input = Pin::new(*pin);
    input.export()?;
    input.set_direction(Direction::In)?;
    input.set_edge(Edge::RisingEdge)?;

    let mut poller = input.get_poller()?;
    loop {
        if let Some(1) = poller.poll(1000)? {
            debug!("pressed...");
            callback();
        }
    }
}
