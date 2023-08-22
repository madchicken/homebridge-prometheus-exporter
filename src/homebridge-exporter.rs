extern crate core;
#[macro_use] extern crate log;

use std::sync::{mpsc};
use std::sync::mpsc::{Sender};
use clap::{Parser};
use log::{debug, LevelFilter};

use tokio::join;
use std::str::FromStr;

use crate::Commands::Exit;
use crate::httpserver::start_metrics_server;

mod httpserver;
mod homebridge;

#[derive(PartialEq)]
pub enum Commands {
    Exit,
}


#[derive(Parser, Debug, Clone)]
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
    #[clap(long, value_parser, default_value = "homebridge")]
    prefix: Option<String>,
    /// Debug mode (displays additional log lines)
    #[clap(long, value_parser, default_value = "false")]
    debug: bool,
    worker_threads: Option<usize>,
    blocking_threads: Option<usize>,
    cpu_threads: Option<usize>,
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config: Config = Config::parse();

    let log_level = if config.debug == true { "debug" } else { "info" };
    env_logger::builder()
        .filter_level(LevelFilter::from_str(log_level).unwrap())
        .init();


    debug!("Parsed command line: {:?}", config);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .thread_name("my-tokio-runtime")
        .enable_all()
        .worker_threads(config.worker_threads.unwrap_or(2))
        .max_blocking_threads(config.blocking_threads.unwrap_or(1))
        .build()
        .unwrap();


    let (tx, _rx) = mpsc::channel::<Commands>();
    let sigkill_loop = runtime.spawn(wait_for_interruption(tx));


    let plot_server_thread = runtime.spawn(start_metrics_server(config));
    runtime.block_on(async {
        let _ = join!(plot_server_thread, sigkill_loop);

        Ok(())
    })
}

async fn wait_for_interruption(tx: Sender<Commands>) {
    match tokio::signal::ctrl_c().await {
        Ok(_) => {
            info!("received Ctrl+C!");
            tx.send(Exit).unwrap();
        }
        Err(_) => {}
    };
}