//! Records a WAV file (roughly 3 seconds long) using the default input device and format.
//!
//! The input data is recorded to "$CARGO_MANIFEST_DIR/recorded.wav".

use std::{sync, thread, time};

use cpal;
use hound;

// use std::{thread, time::Duration, sync::{Arc, sync::atomic::{AtomicBool, Ordering}}};

fn main() {
    // Setup the default input device and stream with the default input format.
    let device = cpal::default_input_device().unwrap();
    let format = device.default_input_format().unwrap();

    println!("Default input format: {:?}", format);
    let event_loop = cpal::EventLoop::new();
    let stream_id = event_loop.build_input_stream(&device, &format).unwrap();
    event_loop.play_stream(stream_id);

    // The WAV file we're recording to.
    const PATH: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
    let writer = hound::WavWriter::create(
        PATH,
        hound::WavSpec {
            channels: format.channels,
            sample_rate: format.sample_rate.0,
            bits_per_sample: (format.data_type.sample_size() * 8) as u16,
            sample_format: hound::SampleFormat::Int,
        },
    ).unwrap();
    let writer = sync::Arc::new(sync::Mutex::new(Some(writer)));

    // A flag to indicate that recording is in progress.
    println!("Begin recording...");
    let recording = sync::Arc::new(sync::atomic::AtomicBool::new(true));

    // Run the input stream on a separate thread.
    let writer_2 = writer.clone();
    let recording_2 = recording.clone();
    thread::spawn(move || {
        event_loop.run(move |_, data| {
            // If we're done recording, return early.
            if !recording_2.load(sync::atomic::Ordering::Relaxed) {
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

    // Let recording go for roughly three seconds.
    thread::sleep(time::Duration::from_secs(5));
    recording.store(false, sync::atomic::Ordering::Relaxed);
    writer.lock().unwrap().take().unwrap().finalize().unwrap();
    println!("Recording {} complete!", PATH);
}
