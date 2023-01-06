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

    /// Homebridge password
    #[clap(short, long, value_parser)]
    password: String,

    /// Homebridge UI uri
    #[clap(long, value_parser, default_value = "http://localhost:8581")]
    uri: String,

    /// Metrics webserver port (service /metrics for Prometheus scraper)
    #[clap(long, value_parser, default_value = "9123")]
    port: u16,

    /// Registry metrics prefix
    #[clap(long, value_parser, default_value = "homebrige")]
    prefix: String,
}


#[tokio::main]
async fn main() {
    let config = Config::parse();
    println!("Parsed command line: {:?}", config);
    let _server = httpserver::start_metrics_server(config).await;
}
