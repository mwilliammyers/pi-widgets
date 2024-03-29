use envy;
use serde_derive::Deserialize;
use std::net::SocketAddr;

// TODO: use `hyper::Uri` when they implement `Deserialize`
#[derive(Clone, Deserialize, Debug)]
pub struct EnvVars {
    #[serde(default = "default_self_address")]
    pub self_address: SocketAddr,

    #[serde(default = "default_gpio_chip")]
    pub gpio_chip: String,

    pub led_line: Option<u32>,
    #[serde(default = "default_led_address")]
    pub led_address: String,
    pub led_button_line: Option<u32>,

    #[serde(default = "default_display_address")]
    pub display_address: String,
    pub display_button_line: Option<u32>,
}

fn default_self_address() -> SocketAddr {
    "0.0.0.0:8080".parse().unwrap()
}

fn default_gpio_chip() -> String {
    "/dev/gpiochip0".to_owned()
}

fn default_led_address() -> String {
    // TODO: for some reason hyper cannot resolve this but curl can...
    // "http://raspberrypi.local:8080/led".to_owned()
    "http://192.168.1.17:8080/led".to_owned()
}

fn default_display_address() -> String {
    // TODO: for some reason hyper cannot resolve this but curl can...
    // "http://macpro.local:8080/led/configure".to_owned()
    "http://192.168.1.169:8080/led/configure".to_owned()
}

pub fn from_env() -> Result<EnvVars, envy::Error> {
    envy::prefixed("WIDGETS_").from_env::<EnvVars>()
}
