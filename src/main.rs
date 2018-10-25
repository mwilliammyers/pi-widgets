use hyper::rt::Future;
use lazy_static::lazy_static;
use log::*;
use regex::Regex;
use serde_derive::Deserialize;
// use serde_json;
use hyper::Uri;
use std::{io, thread};
use warp::{self, path, Filter};

mod button;
mod config;
mod fetch;
mod led;

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(?i).*blink(?\s+the)?\s+light\s+(\d+)\s+for\s+(\d+)\s+ms.*").unwrap();
    static ref CONFIG: config::EnvVars = config::from_env().unwrap();
}

fn main() {
    env_logger::init();

    debug!(
        "self: {}, led: {}, display: {}",
        CONFIG.self_address, CONFIG.led_address, CONFIG.display_address
    );

    let mut led_args = led::BlinkArguments {
        duration_ms: 1000,
        period_ms: 500,
    };

    if let Some(pin) = CONFIG.led_button_pin {
        thread::spawn(move || {
            button::interrupt(pin, || {
                // TODO: make http call instead
                led::blink(&CONFIG.led_pin.unwrap(), &led_args).unwrap()
            })
        });
    }

    if let Some(pin) = CONFIG.display_button_pin {
        thread::spawn(move || {
            button::interrupt(pin, || {

                let fut = fetch::json(CONFIG.display_address.parse().unwrap())
                    .map(|args| {
                        info!("args: {:#?}", args);

                        // led_args.duration_ms = args[0].duration_ms;
                        // led_args.period_ms = args[0].period_ms;
                    }).map_err(|e| match e {
                        fetch::Error::Http(e) => error!("http error: {}", e),
                        fetch::Error::Json(e) => error!("json parsing error: {}", e),
                    });

                warp::spawn(fut);
            })
        });
    }

    let ping = warp::path("ping").map(|| "pong");

    let led_configure = path!("led" / "configure").map(|| {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let cap = RE.captures(&input).unwrap();

        debug!("parsed user input: {:?}", &cap);

        let new_led_args = led::BlinkArguments {
            duration_ms: cap[2].parse().unwrap(),
            // TODO: get this from the user too
            period_ms: 500,
        };

        warp::reply::json(&new_led_args)
    });

    let routes = warp::get2().and(ping).or(warp::post2().and(led_configure));
    warp::serve(routes).run(CONFIG.self_address);
}
