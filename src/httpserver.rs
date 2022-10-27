use std::{
    convert::Infallible
};
use inflector::cases::snakecase::to_snake_case;
use prometheus_client::{encoding::text::encode, registry::Registry};
use prometheus_client::metrics::gauge::Gauge;
use warp::Filter;

use crate::homebridge;
use crate::homebridge::session::{Session};

/// Start a HTTP server to report metrics.
pub async fn start_metrics_server(username: String, password: String, uri: String, port: u16) {
    let session = Session::new(username.to_string(), password.to_string(), uri.to_string()).await;

    let metrics_path = warp::path!("metrics")
        .and_then(move || metrics_get(session.clone(), uri.to_string()));

    println!("Serving /metrics at 127.0.0.1:{}", port);
    warp::serve(metrics_path)
        .run(([127, 0, 0, 1], port))
        .await;

}

async fn metrics_get(session: Session, uri: String) -> Result<impl warp::Reply, Infallible> {
    let mut buf = Vec::new();
    let registry = build_registry(session, uri.to_string()).await;
    encode(&mut buf, &registry).unwrap();
    Ok(warp::reply::with_header(std::str::from_utf8(buf.as_slice()).unwrap().to_string(), "content-type", "application/openmetrics-text; version=1.0.0; charset=utf-8"))
}

async fn build_registry(mut session: Session, uri: String) -> Registry {
    let mut registry = <Registry>::with_prefix("homebridge");
    let token = session.get_token().await;
    let accessories = homebridge::get_all_accessories(&token, uri.to_string()).await.unwrap();

    for accessory in accessories {
        let services = accessory.service_characteristics;
        let values = accessory.values.as_object().to_owned().unwrap();

        for service in services {
            let metric: Gauge<f64> = Gauge::default();
            let metric_name = format!("{}_{}", to_snake_case(&service.service_name.to_string()), to_snake_case(&service.type_.to_string()));

            for key in values.keys() {
                let value = values.get(key).unwrap();
                let value_as_float = value.as_f64().unwrap_or_else(|| 0.0);
                registry.register(
                    metric_name.to_string(),
                    format!("{}", service.description),
                    Box::new(metric.clone()),
                );
                metric.set(value_as_float);

            }
        }
    }
    return registry
}
