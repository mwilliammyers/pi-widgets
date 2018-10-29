use futures::{future, stream::Stream, Future};
use hyper::{
    client::HttpConnector, header, service::service_fn, Body, Client, Method, Request, Response,
    Server, StatusCode,
};
use lazy_static::lazy_static;
use log::*;
use regex::Regex;
use serde_derive::Deserialize;
use serde_json as serde;
use std::{io, io::Write, time::Duration};

mod config;
mod gpio;

type BoxFut = Box<Future<Item = Response<Body>, Error = hyper::Error> + Send>;

static NOT_FOUND: &[u8] = b"Not Found";
static PONG: &[u8] = b"Pong";

lazy_static! {
    static ref CONFIG: config::EnvVars = config::from_env().unwrap();
    static ref BLINK_REGEX: Regex = Regex::new(
        r"(?ix)
        .*
        (?:blink(?:\s+the)?)\s+(?:light|led)
        (?:\s+for\s+(\d+)\s*ms)?
        (?:\s+with\s*a\s*(?:period|freq(?:uency)?)\s*of\s*(\d+)\s*ms)?
        (?:\s+on\s*(\S+))?
        .*"
    ).unwrap();
}

fn route(req: Request<Body>, _client: &Client<HttpConnector>) -> BoxFut {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/led") => Box::new(req.into_body().concat2().and_then(|body| {
            let blink_args = serde::from_slice(&body).unwrap();
            debug!("{:?}", &body);
            gpio::led::blink(&CONFIG.gpio_chip, &CONFIG.led_line.unwrap(), &blink_args).unwrap();

            future::ok(Response::new(Body::empty()))
        })),
        (&Method::GET, "/led/configure") => {
            print!("New configuration? ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            let caps = BLINK_REGEX.captures(&input).unwrap();
            debug!("parsed config: {:?}", &caps);

            let new_led_args = gpio::led::BlinkArguments {
                duration: Duration::from_millis(
                    caps.get(1).map_or("5000", |m| m.as_str()).parse().unwrap(),
                ),
                period: Duration::from_millis(
                    caps.get(2).map_or("250", |m| m.as_str()).parse().unwrap(),
                ),
            };

            Box::new(future::ok(
                Response::builder()
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde::to_string(&new_led_args).unwrap()))
                    .unwrap(),
            ))
        }
        (&Method::GET, "/ping") => Box::new(future::ok(Response::new(Body::from(PONG)))),
        _ => Box::new(future::ok(
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from(NOT_FOUND))
                .unwrap(),
        )),
    }
}

fn main() {
    env_logger::Builder::from_default_env()
        .default_format_timestamp(false)
        .init();

    gpio::init();

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
