use hyper::rt::Future;
use lazy_static::lazy_static;
use log::*;
use regex::Regex;
use serde_derive::Deserialize;
// use serde_json;
use hyper;
use std::{io, io::Write, thread};
use warp::{self, path, Filter};

mod button;
mod config;
mod fetch;
mod led;

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(?i).*blink(?:\s+the)?\s+light\s+for\s+(\d+)\s*ms.*").unwrap();
    static ref CONFIG: config::EnvVars = config::from_env().unwrap();
}

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    debug!(
        "self: {}, led: {}, display: {}",
        CONFIG.self_address, CONFIG.led_address, CONFIG.display_address
    );

    thread::spawn(|| {
        let mut led_args = led::BlinkArguments {
            duration_ms: 1000,
            period_ms: 500,
        };

        let mut led_button = button::get_poller(&CONFIG.led_button_pin).unwrap();

        let mut display_button = button::get_poller(&CONFIG.display_button_pin).unwrap();

        loop {
            if let Some(1) = &led_button.poll(1000).unwrap() {
                led::blink(&CONFIG.led_pin, &led_args).unwrap();
            }

            if let Some(1) = &display_button.poll(1000).unwrap() {
                let fut = fetch::json(CONFIG.display_address.parse().unwrap())
                    .map(move |args| {
                        info!("args: {:?}", args);

                        led_args = args[0];
                        // led_args.duration_ms = args[0].duration_ms;
                        // led_args.period_ms = args[0].period_ms;
                    }).map_err(|e| match e {
                        fetch::Error::Http(e) => error!("http error: {}", e),
                        fetch::Error::Json(e) => error!("json parsing error: {}", e),
                    }).nth(0)

                hyper::rt::run(fut);
            }
        }
    });

    let ping = warp::path("ping").map(|| "pong");

    let led_configure = path!("led" / "configure").map(|| {
        print!("New configuration? ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        debug!("user input: {}", &input);

        let cap = RE.captures(&input).unwrap();
        debug!("parsed user input: {:?}", &cap);

        let new_led_args = led::BlinkArguments {
            duration_ms: cap[1].parse().unwrap(),
            // TODO: get this from the user too
            period_ms: 500,
        };

        warp::reply::json(&[new_led_args])
    });

    let routes = warp::get2()
        .and(ping.or(led_configure))
        .or(warp::post2().and(led_configure));
    warp::serve(routes).run(CONFIG.self_address);
}
