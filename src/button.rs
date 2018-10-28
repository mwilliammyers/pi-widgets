use log::*;
// use sysfs_gpio::{Direction, Edge, Pin};

use gpio_cdev::*;
use nix::poll::*;
use std::os::unix::io::AsRawFd;

type PollEventFlags = nix::poll::EventFlags;

pub fn interrupt<F>(chip: &String, lines: &Vec<u32>, mut callback: F) -> errors::Result<()>
where
    F: FnMut(u32, gpio_cdev::LineEvent) + Send + 'static,
{
    let mut chip = Chip::new(chip)?;

    // Get event handles for each line to monitor.
    let mut evt_handles: Vec<LineEventHandle> = lines
        .into_iter()
        .map(|off| {
            let line = chip.get_line(*off).unwrap();
            line.events(
                LineRequestFlags::INPUT,
                EventRequestFlags::RISING_EDGE,
                "monitor",
            ).unwrap()
        }).collect();

    // Create a vector of file descriptors for polling
    let mut pollfds: Vec<PollFd> = evt_handles
        .iter()
        .map(|h| {
            PollFd::new(
                h.as_raw_fd(),
                PollEventFlags::POLLIN | PollEventFlags::POLLPRI,
            )
        }).collect();

    loop {
        // poll for an event on any of the lines
        if poll(&mut pollfds, -1)? == 0 {
            error!("timeout");
        } else {
            for i in 0..pollfds.len() {
                if let Some(revts) = pollfds[i].revents() {
                    let h = &mut evt_handles[i];
                    if revts.contains(PollEventFlags::POLLIN) {
                        let event = h.get_event()?;

                        debug!("[{}] {:?}", h.line().offset(), event);

                        callback(h.line().offset(), event);
                    } else if revts.contains(PollEventFlags::POLLPRI) {
                        error!("[{}] got a POLLPRI", h.line().offset());
                    }
                }
            }
        }
    }
}
