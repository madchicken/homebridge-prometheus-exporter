use std::process::exit;
use inflector::cases::snakecase::to_snake_case;
use prometheus_client::{encoding::text::encode, registry::Registry};
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use warp::{Filter, Rejection};
use warp::reject::Reject;

use crate::{Config, homebridge};
use crate::homebridge::session::{Session};

/// Start a HTTP server to report metrics.
pub async fn start_metrics_server(config: Config) {
    let session = Session::new(config.username.to_string(), config.password.to_string(), config.uri.to_string()).await;
    let option = session.token.clone();
    match option {
        Some(_t) => {
            let metrics_path = warp::path!("metrics")
                .and_then(move || metrics_get(session.clone(), config.uri.to_string(), config.prefix.to_string()));

            println!("Serving /metrics at 127.0.0.1:{}", config.port);
            warp::serve(metrics_path)
                .run(([127, 0, 0, 1], config.port))
                .await;
        }
        None => {
            println!("There was an error while initializing Homebridge APIs. Please double check the username, the password and the uri you provided as arguments to this program.");
            exit(1)
        }
    }
}

#[derive(Debug)]
struct GenericError;

impl Reject for GenericError {}

async fn metrics_get(session: Session, uri: String, prefix: String) -> Result<impl warp::Reply, Rejection> {
    let mut buf = Vec::new();
    let result = build_registry(session, uri.to_string(), prefix.to_string()).await;
    match result {
        Ok(registry) => {
            encode(&mut buf, &registry).unwrap();
            Ok(warp::reply::with_header(std::str::from_utf8(buf.as_slice()).unwrap().to_string(), "content-type", "application/openmetrics-text; version=1.0.0; charset=utf-8"))
        }
        Err(_e) => Err(warp::reject::custom(GenericError))
    }
}

async fn build_registry(mut session: Session, uri: String, prefix: String) -> Result<Registry, String> {
    let mut registry = <Registry>::with_prefix(prefix.as_str());
    let result = session.get_token().await;
    match result {
        Ok(token) => {
            let accessories_result = homebridge::get_all_accessories(&token, uri.to_string()).await;
            match accessories_result {
                Ok(accessories) => {
                    for accessory in accessories {
                        let services = accessory.service_characteristics;

                        for service in services {
                            if !service.format.eq_ignore_ascii_case("string") { // ignore string service types
                                let metric = Family::<Vec<(String, String)>, Gauge<f64>>::default();
                                let metric_name = format!("{}_{}", to_snake_case(&service.service_type.to_string()), to_snake_case(&service.type_.to_string()));
                                let value_as_float = service.value.as_f64().unwrap_or_else(|| 0.0);
                                registry.register(
                                    metric_name.to_string(),
                                    format!("{}", service.description),
                                    Box::new(metric.clone()),
                                );

                                metric.get_or_create(&vec![("name".to_owned(), to_snake_case(&service.service_name.to_string()).to_owned())]).set(value_as_float);
                            }
                        }
                    }
                    Ok(registry)
                }
                Err(e) => {
                    println!("{}", e);
                    Err(format!("{}", e))
                }
            }

        }
        Err(e) => {
            println!("{}", e);
            Err(format!("{}", e))
        }
    }
}
