extern crate env_logger;
extern crate warp;

use warp::Filter;

fn main() {
    env_logger::init();

    let ping = warp::path("ping").map(|| "Pong");

    let routes = warp::get2().and(ping);

    warp::serve(routes).run(([127, 0, 0, 1], 8080));
}
