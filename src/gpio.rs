use futures::{Future, Stream};
use hyper::{self, Body, Client, Request, Uri};
use log::*;
use std::{fs, thread, vec::Vec};

pub mod led {
    use gpio_cdev::*;
    use log::*;
    use serde_derive::{Deserialize, Serialize};
    use std::{
        thread,
        time::{Duration, Instant},
    };

    #[derive(Serialize, Deserialize, Debug)]
    pub struct BlinkArguments {
        #[serde(default = "default_duration")]
        pub duration: Duration,
        #[serde(default = "default_period")]
        pub period: Duration,
    }

    fn default_duration() -> Duration {
        Duration::from_millis(4000)
    }

    fn default_period() -> Duration {
        Duration::from_millis(500)
    }

    pub fn blink(chip: &String, line: &u32, args: &BlinkArguments) -> errors::Result<()> {
        debug!("{:?}", &args);

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
}

mod button {
    use gpio_cdev::*;
    use log::*;
    use nix::poll::*;
    use std::os::unix::io::AsRawFd;

    type PollEventFlags = nix::poll::EventFlags;

    pub fn interrupt<F>(chip: &String, lines: &Vec<u32>, callback: F) -> errors::Result<()>
    where
        F: Fn(u32, gpio_cdev::LineEvent),
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
            if poll(&mut pollfds, -1)? == 0 {
                error!("timeout");
                continue;
            }

            for (i, fd) in pollfds.iter().enumerate() {
                if let Some(revts) = fd.revents() {
                    let h = &mut evt_handles[i];

                    if revts.contains(PollEventFlags::POLLPRI) {
                        error!("[{}] got a POLLPRI", h.line().offset());
                        continue;
                    }

                    if revts.contains(PollEventFlags::POLLIN) {
                        let event = h.get_event()?;

                        debug!("[{}] {:?}", h.line().offset(), event);

                        callback(h.line().offset(), event);
                    }
                }
            }
        }
    }
}

use crate::CONFIG;

// TODO: pass in CONFIG...
pub fn init() {
    let mut gpio_lines = Vec::new();
    for line in [CONFIG.led_button_line, CONFIG.display_button_line].iter() {
        if let Some(l) = *line {
            gpio_lines.push(l);
        }
    }

    if !gpio_lines.is_empty() {
        thread::spawn(move || {
            button::interrupt(&CONFIG.gpio_chip, &gpio_lines, |line, _event| {
                let led_addr: Uri = CONFIG.led_address.parse().unwrap();
                let display_addr: Uri = CONFIG.display_address.parse().unwrap();

                if let Some(led_button_line) = CONFIG.led_button_line {
                    if line == led_button_line {
                        let body = Body::from(fs::read("/tmp/widgets.json").unwrap());
                        let fut = Client::new()
                            .request(Request::post(led_addr).body(body).unwrap())
                            .map(|res| debug!("{}", res.status()))
                            .map_err(|err| error!("{}", err));

                        // TODO: use existing rt?
                        hyper::rt::run(fut);
                    }
                }

                if let Some(display_button_line) = CONFIG.display_button_line {
                    if line == display_button_line {
                        // TODO: share client?
                        let fut = Client::new()
                            .get(display_addr)
                            .and_then(|res| res.into_body().concat2())
                            .and_then(|body| {
                                fs::write("/tmp/widgets.json", body).unwrap();
                                Ok(())
                            }).map_err(|err| error!("{}", err));

                        // TODO: use existing rt?
                        hyper::rt::run(fut);
                    }
                }
            }).unwrap();
        });
    }
}
