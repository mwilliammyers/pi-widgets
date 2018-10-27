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
}

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    let CONFIG = config::from_env().unwrap().clone();

    let mut led_args = led::BlinkArguments {
        duration_ms: 4000,
        period_ms: 750,
    };

    if let Some(ref pin) = &CONFIG.led_button_pin {
        button::interrupt(&pin, |level| {
            // TODO: make http call instead
            led::blink(&CONFIG.led_pin.unwrap(), &led_args);
        });
    }

    if let Some(ref pin) = CONFIG.display_button_pin {
        // thread::spawn(|| {
        button::interrupt(pin, |level| {
            let fut = fetch::json(CONFIG.display_address.parse().unwrap())
                .map(|args| {
                    info!("args: {:?}", args);
                    led_args = args[0];
                }).map_err(|e| match e {
                    fetch::Error::Http(e) => error!("http error: {}", e),
                    fetch::Error::Json(e) => error!("json parsing error: {}", e),
                });

            hyper::rt::run(fut);
        })
        // });
    }

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
