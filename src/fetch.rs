use crate::led::BlinkArguments;
use hyper::{
    self,
    rt::{Future, Stream},
    Client,
};
use log::*;
use serde_json;

pub fn json(url: hyper::Uri) -> impl Future<Item = Vec<BlinkArguments>, Error = Error> {
    debug!("making request to: {}...", &url);

    let client = Client::new();

    client
        .get(url)
        .and_then(|res| res.into_body().concat2())
        .from_err::<Error>()
        .and_then(|body| {
            debug!("received response");

            let args = serde_json::from_slice(&body)?;
            Ok(args)
        }).from_err()
}

pub enum Error {
    Http(hyper::Error),
    Json(serde_json::Error),
}

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Error {
        Error::Http(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::Json(err)
    }
}
