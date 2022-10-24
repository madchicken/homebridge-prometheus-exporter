use clap::{Parser};

mod httpserver;
mod homebridge;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Config {
    /// Homebridge username
    #[clap(short, long, value_parser)]
    username: String,

    /// Homebridge username
    #[clap(short, long, value_parser)]
    password: String,

    /// Homebridge uri
    #[clap(long, value_parser, default_value = "http://192.168.0.2:8080")]
    uri: String,

    #[clap(long, value_parser, default_value = "8001")]
    port: u16,
}


#[tokio::main]
async fn main() {
    let config = Config::parse();
    httpserver::start_metrics_server(config.username, config.password, config.uri, config.port).await
}
