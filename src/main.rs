use lazy_static::lazy_static;
use log::*;
use regex::Regex;
use serde_derive::Deserialize;
// use serde_json;
use futures::{future, Future};
use hyper::{
    client::HttpConnector, header, service::service_fn, Body, Client, Method, Request, Response,
    Server, StatusCode,
};
use std::{io, io::Write, thread, time::Duration, vec::Vec};

mod button;
mod config;
mod fetch;
mod led;

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

static NOTFOUND: &[u8] = b"Not Found";
static PONG: &[u8] = b"Pong";

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(?i).*blink(?:\s+the)?\s+light\s+for\s+(\d+)\s*ms.*").unwrap();
    static ref CONFIG: config::EnvVars = config::from_env().unwrap();
}

fn route(req: Request<Body>, _client: &Client<HttpConnector>) -> BoxFut {
    let (parts, _body) = req.into_parts();

    match (parts.method, parts.uri.path()) {
        (Method::GET, "/ping") => Box::new(future::ok(Response::new(Body::from(PONG)))),
        (Method::GET, "/led/configure") => {
            print!("New configuration? ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            debug!("user input: {}", &input);

            let cap = RE.captures(&input).unwrap();
            debug!("parsed user input: {:?}", &cap);

            let new_led_args = led::BlinkArguments {
                duration: Duration::from_millis(cap[1].parse().unwrap()),
                // TODO: get this from the user too
                period: Duration::from_millis(500),
            };

            Box::new(future::ok(
                Response::builder()
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_string(&new_led_args).unwrap()))
                    .unwrap(),
            ))
        }
        _ => Box::new(future::ok(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(NOTFOUND))
                .unwrap(),
        )),
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    let mut gpio_lines = Vec::new();
    for line in &[CONFIG.led_button_line, CONFIG.display_button_line] {
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

            button::interrupt(&CONFIG.gpio_chip, &gpio_lines, move |line, _event| {
                if let Some(led_button_line) = CONFIG.led_button_line {
                    if line == led_button_line {
                        // TODO: make http call instead
                        led::blink(&CONFIG.gpio_chip, &CONFIG.led_line.unwrap(), &led_args).unwrap()
                    }
                }

                // if let Some(display_button_line) = CONFIG.display_button_line {
                //     if line == display_button_line {
                //         let fut = fetch::json(CONFIG.display_address.parse().unwrap())
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
            })
        });
    }

    hyper::rt::run(future::lazy(move || {
        let client = Client::new();

        info!("listening on http://{}", &CONFIG.self_address);
        Server::bind(&CONFIG.self_address)
            .serve(move || {
                let client = client.clone();
                service_fn(move |req| route(req, &client))
            }).map_err(|e| error!("server error: {}", e))
    }));
}
