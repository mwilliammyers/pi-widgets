use std::{thread, time::Duration, vec::Vec};

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
        pub duration: Duration,
        pub period: Duration,
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

pub fn init(
    gpio_chip: String,
    led_line: Option<u32>,
    led_button_line: Option<u32>,
    display_button_line: Option<u32>,
) {
    let mut gpio_lines = Vec::new();
    for line in [led_button_line, display_button_line].iter() {
        if let Some(l) = *line {
            gpio_lines.push(l);
        }
    }

    if !gpio_lines.is_empty() {
        thread::spawn(move || {
            let mut led_args = led::BlinkArguments {
                duration: Duration::from_millis(1000),
                period: Duration::from_millis(250),
            };

            button::interrupt(&gpio_chip, &gpio_lines, |line, _event| {
                if let Some(led_button_line) = led_button_line {
                    if line == led_button_line {
                        // TODO: make http call instead
                        led::blink(&gpio_chip, &led_line.unwrap(), &led_args).unwrap()
                    }
                }

                // if let Some(display_button_line) = config.display_button_line {
                //     if line == display_button_line {
                //         let fut = fetch::json(config.display_address.parse().unwrap())
                //             .map(|args| {
                //                 info!("args: {:?}", args);

                //                 led_args.duration = args[0].duration;
                //                 led_args.period = args[0].period;
                //             }).map_err(|e| match e {
                //                 fetch::Error::Http(e) => error!("http error: {}", e),
                //                 fetch::Error::Json(e) => error!("json parsing error: {}", e),
                //             });

                //         hyper::rt::run(fut);
                //     }
                // }
            }).unwrap();
        });
    }
}
