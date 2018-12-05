use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{fs, thread, time::Duration, vec::Vec};

use futures::{Future, Stream};
use hyper::{self, Body, Client, Request, Uri};
use log::*;

use cpal;
use hound;

pub mod led {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use gpio_cdev::{errors, Chip, LineRequestFlags};
    use log::*;
    use serde_derive::{Deserialize, Serialize};

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
        loop {
            if start_time.elapsed() >= args.duration {
                break;
            }

            handle.set_value(1)?;
            thread::sleep(args.period);
            handle.set_value(0)?;
            thread::sleep(args.period);
        }

        Ok(())
    }
}

mod button {
    use std::os::unix::io::AsRawFd;

    use gpio_cdev::{errors, Chip, EventRequestFlags, LineEventHandle, LineRequestFlags};
    use log::*;
    use nix::poll::{poll, EventFlags, PollFd};

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
            .map(|h| PollFd::new(h.as_raw_fd(), EventFlags::POLLIN))
            .collect();

        loop {
            if poll(&mut pollfds, -1)? == 0 {
                error!("timeout");
                continue;
            }

            for (i, fd) in pollfds.iter().enumerate() {
                let h = &mut evt_handles[i];

                if let Some(revts) = fd.revents() {
                    if revts.contains(EventFlags::POLLIN) {
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

// TODO: refactor: pass in config? or accept a hashmap of buttons to functions?
pub fn init() {
    let all_gpio_lines = [CONFIG.led_button_line, CONFIG.display_button_line];
    let gpio_lines: Vec<u32> = all_gpio_lines.iter().filter_map(|line| *line).collect();

    if gpio_lines.is_empty() {
        return;
    }

    debug!("setting up gpio_lines: {:?}", &gpio_lines);
    thread::spawn(move || {
        button::interrupt(&CONFIG.gpio_chip, &gpio_lines, |line, _event| {
            let led_addr: Uri = CONFIG.led_address.parse().unwrap();
            let display_addr: Uri = CONFIG.display_address.parse().unwrap();
            let led_config = &CONFIG.led_config;
            let audio_recording = &CONFIG.audio_recording;

            if let Some(led_button_line) = CONFIG.led_button_line {
                if line == led_button_line {
                    let body = Body::from(fs::read(led_config).unwrap());
                    // TODO: share client from outside thread?
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
                    // Setup the default input device and stream with the default input format.
                    let device = cpal::default_input_device().unwrap();
                    let format = device.default_input_format().unwrap();

                    debug!("Default input format: {:?}", format);
                    let event_loop = cpal::EventLoop::new();
                    let stream_id = event_loop.build_input_stream(&device, &format).unwrap();
                    event_loop.play_stream(stream_id);

                    // The WAV file we're recording to.
                    let writer = hound::WavWriter::create(
                        &audio_recording,
                        hound::WavSpec {
                            channels: format.channels,
                            sample_rate: format.sample_rate.0,
                            bits_per_sample: (format.data_type.sample_size() * 8) as u16,
                            sample_format: hound::SampleFormat::Int,
                        },
                    ).unwrap();
                    let writer = Arc::new(Mutex::new(Some(writer)));

                    // A flag to indicate that recording is in progress.
                    let recording = Arc::new(AtomicBool::new(true));

                    // Run the input stream on a separate thread.
                    let writer_2 = writer.clone();
                    let recording_2 = recording.clone();
                    thread::spawn(move || {
                        event_loop.run(move |_, data| {
                            // If we're done recording, return early.
                            if !recording_2.load(Ordering::Relaxed) {
                                return;
                            }
                            // Otherwise write to the wav writer.
                            match data {
                                cpal::StreamData::Input {
                                    buffer: cpal::UnknownTypeInputBuffer::U16(buffer),
                                } => {
                                    if let Ok(mut guard) = writer_2.try_lock() {
                                        if let Some(writer) = guard.as_mut() {
                                            for sample in buffer.iter() {
                                                let sample = cpal::Sample::to_i16(sample);
                                                writer.write_sample(sample).ok();
                                            }
                                        }
                                    }
                                }
                                cpal::StreamData::Input {
                                    buffer: cpal::UnknownTypeInputBuffer::I16(buffer),
                                } => {
                                    if let Ok(mut guard) = writer_2.try_lock() {
                                        if let Some(writer) = guard.as_mut() {
                                            for &sample in buffer.iter() {
                                                writer.write_sample(sample).ok();
                                            }
                                        }
                                    }
                                }
                                cpal::StreamData::Input {
                                    buffer: cpal::UnknownTypeInputBuffer::F32(buffer),
                                } => {
                                    if let Ok(mut guard) = writer_2.try_lock() {
                                        if let Some(writer) = guard.as_mut() {
                                            for sample in buffer.iter() {
                                                let sample = cpal::Sample::to_i16(sample);
                                                writer.write_sample(sample).ok();
                                            }
                                        }
                                    }
                                }
                                _ => (),
                            }
                        });
                    });

                    thread::sleep(Duration::from_secs(7));
                    recording.store(false, Ordering::Relaxed);

                    writer.lock().unwrap().take().unwrap().finalize().unwrap();
                    debug!("Recording complete!");
                }
            }
        }).unwrap();
    });
}
