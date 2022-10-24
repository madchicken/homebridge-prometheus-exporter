use std::{
    convert::Infallible
};
use inflector::cases::snakecase::to_snake_case;
use prometheus_client::{encoding::text::encode, registry::Registry};
use prometheus_client::metrics::gauge::Gauge;
use serde_json::Value;
use warp::Filter;

use crate::homebridge;

/// Start a HTTP server to report metrics.
pub async fn start_metrics_server(username: String, password: String, uri: String, port: u16) {
    let metrics_path = warp::path!("metrics")
        .and_then(move || metrics_get(username.to_string(), password.to_string(), uri.to_string()));

    warp::serve(metrics_path)
        .run(([127, 0, 0, 1], port))
        .await;

}

async fn metrics_get(username: String, password: String, uri: String) -> Result<impl warp::Reply, Infallible> {
    let mut buf = Vec::new();
    let registry = build_registry(username.to_string(), password.to_string(), uri.to_string()).await;
    encode(&mut buf, &registry).unwrap();
    Ok(warp::reply::with_header(std::str::from_utf8(buf.as_slice()).unwrap().to_string(), "content-type", "application/openmetrics-text; version=1.0.0; charset=utf-8"))
}

async fn build_registry(username: String, password: String, uri: String) -> Registry {
    let mut registry = <Registry>::with_prefix("homebridge");
    let token = homebridge::login(username, password, uri.to_string()).await.unwrap();
    let accessories = homebridge::get_all_accessories(&token, uri.to_string()).await.unwrap();

    for accessory in accessories {
        println!("Accessory: type {}, name {}", accessory["type"], accessory["serviceName"]);
        let services: &Vec<Value> = accessory["serviceCharacteristics"].as_array().unwrap();
        let values = accessory["values"].as_object().to_owned().unwrap();
        println!("Values: {:?}", values);

        for service in services {
            let metric: Gauge<f64> = Gauge::default();
            let metric_name = format!("{}_{}", to_snake_case(&service["serviceName"].to_string()), to_snake_case(&service["type"].to_string()));

            let value = values.get(service["type"].as_str().unwrap()).unwrap_or_else(|| &Value::Null);
            if *value != Value::Null {
                let value_as_float = value.as_f64().unwrap_or_else(|| 0.0);
                registry.register(
                    metric_name,
                    format!("{}", service["description"]),
                    Box::new(metric.clone()),
                );
                metric.set(value_as_float);
            }
        }
    }
    return registry;
}
