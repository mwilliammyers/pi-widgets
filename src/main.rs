use futures::{future, Future};
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

            let caps = RE.captures(&input).unwrap();
            debug!("parsed user input: {:?}", &caps);

            let new_led_args = gpio::led::BlinkArguments {
                duration: Duration::from_millis(caps[1].parse().unwrap()),
                // TODO: get this from the user too
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

    hyper::rt::run(future::lazy(move || {
        let client = Client::new();

        let config = config::from_env().unwrap();

        gpio::init(
            config.gpio_chip,
            config.led_line,
            config.led_button_line,
            config.display_button_line,
        );

        info!("listening on http://{}", &config.self_address);
        Server::bind(&config.self_address)
            .serve(move || {
                let client = client.clone();
                service_fn(move |req| route(req, &client))
            }).map_err(|e| error!("server error: {}", e))
    }));
}
