extern crate core;
#[macro_use] extern crate log;

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
    /// Authorization keys file. Default to authorization-keys.yml in the current working directory.
    #[clap(long, value_parser, default_value = "authorization-keys.yml")]
    keyfile: String,
    /// Metrics webserver port (service /metrics for Prometheus scraper)
    #[clap(long, value_parser, default_value = "9123")]
    port: u16,
    /// Registry metrics prefix
    #[clap(long, value_parser, default_value = "homebrige")]
    prefix: String,
    /// Debug mode (displays additional log lines)
    #[clap(long, value_parser, default_value = "false")]
    debug: bool,
}


#[actix_web::main]
async fn main() {
    let config: Config = Config::parse();
    let level = if config.debug == true { "debug" } else { "info" };
    std::env::set_var("RUST_LOG", level);
    env_logger::init();
    debug!("Parsed command line: {:?}", config);
    let _server = httpserver::start_metrics_server(config).await;
}
