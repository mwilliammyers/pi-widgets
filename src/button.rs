use log::*;
use sysfs_gpio::{Direction, Edge, Pin, PinPoller};

pub fn get_poller(pin: &u64) -> sysfs_gpio::Result<PinPoller> {
    let input = Pin::new(*pin);
    input.export()?;
    input.set_direction(Direction::In)?;
    input.set_edge(Edge::RisingEdge)?;
    input.get_poller()
}
