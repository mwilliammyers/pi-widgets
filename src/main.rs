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

static NOTFOUND: &[u8] = b"Not Found";
static PONG: &[u8] = b"Pong";

lazy_static! {
    static ref RE: Regex =
        Regex::new(r"(?i).*blink(?:\s+the)?\s+light\s+for\s+(\d+)\s*ms.*").unwrap();
    static ref CONFIG: config::EnvVars = config::from_env().unwrap();
}

fn route(req: Request<Body>, _client: &Client<HttpConnector>) -> BoxFut {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/ping") => Box::new(future::ok(Response::new(Body::from(PONG)))),
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
            debug!("user input: {}", &input);

            let caps = RE.captures(&input).unwrap();
            debug!("parsed user input: {:?}", &caps);

            let new_led_args = gpio::led::BlinkArguments {
                duration: Duration::from_millis(caps[1].parse().unwrap()),
                // TODO: get from user too
                period: Duration::from_millis(500),
            };

            Box::new(future::ok(
                Response::builder()
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde::to_string(&new_led_args).unwrap()))
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
