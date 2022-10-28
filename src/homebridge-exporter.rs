extern crate core;

use clap::{Parser};

mod httpserver;
mod homebridge;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    /// Homebridge username
    #[clap(short, long, value_parser)]
    username: String,

    /// Homebridge username
    #[clap(short, long, value_parser)]
    password: String,

    /// Homebridge uri
    #[clap(long, value_parser, default_value = "http://localhost")]
    uri: String,

    #[clap(long, value_parser, default_value = "8581")]
    port: u16,

    /// metrics prefix
    #[clap(long, value_parser, default_value = "homebrige")]
    prefix: String,
}


#[tokio::main]
async fn main() {
    let config = Config::parse();
    httpserver::start_metrics_server(config).await
}
