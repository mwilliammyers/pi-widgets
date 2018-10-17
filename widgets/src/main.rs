#[macro_use]
extern crate log;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate env_logger;
extern crate sysfs_gpio;
extern crate warp;
#[macro_use]
extern crate lazy_static;
extern crate regex;

use regex::Regex;
use std::thread::sleep;
use std::time::Duration;
use sysfs_gpio::{Direction, Pin};
use warp::Filter;

#[derive(Deserialize, Serialize)]
struct RequestBody {
    message: String,
}

fn blink_led(pin_number: u64, duration_ms: u64, period_ms: u64) -> sysfs_gpio::Result<()> {
    info!(
        "blinking led: {} for {}ms with a period of {}ms",
        &pin_number, &duration_ms, &period_ms
    );

    let led = Pin::new(pin_number);
    led.with_exported(|| {
        // TODO: replace this with a check to see if it is ready
        // sleep(Duration::from_millis(200));

        led.set_direction(Direction::Low)?;
        let iterations = duration_ms / period_ms / 2;
        for _ in 0..iterations {
            led.set_value(0)?;
            sleep(Duration::from_millis(period_ms));
            led.set_value(1)?;
            sleep(Duration::from_millis(period_ms));
        }
        led.set_value(0)?;
        Ok(())
    })
}

fn main() {
    env_logger::init();

    let ping = warp::path("ping").map(|| "pong");

    let led = warp::path("led")
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::json())
        .map(|body: RequestBody| {
            info!("received LED request: {}", &body.message);

            lazy_static! {
                static ref RE: Regex = Regex::new(
                    r"(?ix).*
                    blink\s*
                    light\s*
                    (?:num(?:ber))?\s*
                    (\d+)\s*
                    for\s*
                    (\d+)\s*ms.*"
                ).unwrap();
            }

            let cap = RE.captures(&body.message).unwrap();
            debug!("parsed request: {:?}", &cap);

            blink_led(
                cap[1].parse::<u64>().unwrap(),
                cap[2].parse::<u64>().unwrap(),
                500,
            ).unwrap();

            "success"
        });

    let routes = warp::get2().and(ping).or(warp::post2().and(led));

    warp::serve(routes).run(([0, 0, 0, 0], 8080));
}
