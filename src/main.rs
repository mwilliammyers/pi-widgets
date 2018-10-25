use lazy_static::lazy_static;
use log::*;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::{env, net::SocketAddr, thread, time::Duration};
use sysfs_gpio::{Direction, Edge, Pin};
use warp::Filter;

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(?i).*blink(?\s+the)?\s+light\s+(\d+)\s+for\s+(\d+)\s+ms.*").unwrap();
}

#[derive(Deserialize, Debug)]
struct Config {
    led_pin: Option<u64>,
    button_pin: Option<u64>,
    self_address: SocketAddr,
    led_address: SocketAddr,
    led_button_address: SocketAddr,
    // config_button_address: SocketAddr,
    display_address: SocketAddr,
}

#[derive(Deserialize, Serialize)]
struct RequestBody {
    message: String,
}

fn blink_led(pin: u64, duration_ms: u64, period_ms: u64) -> sysfs_gpio::Result<()> {
    info!(
        "blinking led: {} for {}ms with a period of {}ms",
        &pin, &duration_ms, &period_ms
    );

    let led = Pin::new(pin);
    led.with_exported(|| {
        led.set_direction(Direction::Low)?;

        let iterations = duration_ms / period_ms / 2;
        for _ in 0..iterations {
            led.set_value(0)?;
            thread::sleep(Duration::from_millis(period_ms));
            led.set_value(1)?;
            thread::sleep(Duration::from_millis(period_ms));
        }
        led.set_value(0)?;

        Ok(())
    })
}

fn interrupt(pin: u64, callback: fn()) -> sysfs_gpio::Result<()> {
    let input = Pin::new(pin);
    input.with_exported(|| {
        input.set_direction(Direction::In)?;
        input.set_edge(Edge::RisingEdge)?;

        let mut poller = input.get_poller()?;
        loop {
            if let Some(1) = poller.poll(1000)? {
                callback();
            }
        }
    })
}

fn main() {
    env_logger::init();

    let config = envy::prefixed("WIDGETS_").from_env::<Config>().unwrap();

    if let Some(pin) = config.button_pin {
        thread::spawn(|| interrupt(pin, || info!("button pressed!")));
    }

    let ping = warp::path("ping").map(|| "pong");

    let led = warp::path("led")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .map(|body: RequestBody| {
            info!("received LED request: {}", &body.message);

            if let Some(pin) = config.led_pin {
                let cap = RE.captures(&body.message).unwrap();
                debug!("parsed request: {:?}", &cap);

                blink_led(pin, cap[2].parse::<u64>().unwrap(), 500).unwrap();
            }

            "success"
        });

    let routes = warp::get2().and(ping).or(warp::post2().and(led));

    warp::serve(routes).run(config.self_address);
}
