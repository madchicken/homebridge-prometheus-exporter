use std::sync::Mutex;
use inflector::cases::snakecase::to_snake_case;
use prometheus_client::{encoding::text::encode, registry::Registry};
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use actix_web::{web, App, HttpServer, HttpResponse, web::Data, HttpRequest};
use actix_web::http::StatusCode;

use crate::{Config, homebridge};
use crate::homebridge::session::{Session};

use serde::{Deserialize, Serialize};
use serde_yaml::{self};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AuthorizationKeys {
    keys: Vec<String>,
}

fn load_keys() -> AuthorizationKeys {
    let f = std::fs::File::open("authorization-keys.yml").expect("Could not open file.");
    let keys: AuthorizationKeys = serde_yaml::from_reader(f).expect("Could not read values.");

    println!("{:?}", keys);
    return keys;
}

/// Start a HTTP server to report metrics.
pub async fn start_metrics_server(config: Config) -> std::io::Result<()> {
    println!("Creating session");
    let port = config.port;
    let uri = config.uri.clone();
    let password = config.password.clone();
    let username = config.username.clone();
    let shared_config = Data::new(config);
    let session: Session = Session::new(username, password, uri);
    println!("Session created {:?}", session);
    let shared_session = Data::new(Mutex::new(session));
    let keys: AuthorizationKeys = load_keys();

    println!("Serving /metrics at 127.0.0.1:{}", port);
    HttpServer::new(move || {
        App::new()
            .app_data(shared_session.clone())
            .app_data(shared_config.clone())
            .app_data(keys.clone())
            .service(web::resource("/metrics").route(web::get().to(metrics_get)))
            .service(web::resource("/restart").route(web::get().to(restart)))
    })
        .bind(("127.0.0.1", port))?
        .run()
        .await
}

fn check_bearer_token(req: HttpRequest, keys: Data<AuthorizationKeys>) -> bool {
    if req.headers().contains_key("Authorization") {
        let bearer = req.headers().get("Authorization").unwrap().to_str().unwrap();
        let parts: Vec<_> = bearer.split(' ').collect();
        if parts[0].eq("Bearer") {
            let req_key = parts[1];
            let index = keys.keys
                .iter()
                .position(|key| key.eq(req_key));
            return match index {
                Some(_) => true,
                None => false
            }
        }
        return false;
    }
    return false;
}

async fn restart(session: Data<Mutex<Session>>, config: Data<Config>, keys: Data<AuthorizationKeys>, req: HttpRequest) -> actix_web::Result<HttpResponse> {
    match check_bearer_token(req, keys) {
        true => {
            let token = session.lock().unwrap().get_token().await.unwrap();
            let result = homebridge::restart(token, config.uri.clone()).await;
            match result {
                Ok(_b) => Ok(HttpResponse::build(StatusCode::OK).body("done")),
                Err(e) => Ok(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body(e))
            }
        }
        false => Ok(HttpResponse::build(StatusCode::UNAUTHORIZED).body("Unauthorized request, please provide a valid token."))
    }
}

async fn metrics_get(session: Data<Mutex<Session>>, config: Data<Config>, _keys: Data<AuthorizationKeys>, _req: HttpRequest) -> actix_web::Result<HttpResponse> {
    let mut buf = Vec::new();
    let token = session.lock().unwrap().get_token().await.unwrap();
    let result = build_registry(token, config.uri.clone(), config.prefix.clone()).await;
    match result {
        Ok(registry) => {
            encode(&mut buf, &registry).unwrap();
            Ok(HttpResponse::build(StatusCode::OK)
                   .content_type("application/openmetrics-text; version=1.0.0; charset=utf-8")
                   .body(std::str::from_utf8(buf.as_slice()).unwrap().to_string()))
        }
        Err(e) => Ok(HttpResponse::build(StatusCode::INTERNAL_SERVER_ERROR).body(e))
    }
}

async fn build_registry(token: String, uri: String, prefix: String) -> Result<Registry, String> {
    let mut registry = <Registry>::with_prefix(prefix.as_str());
    let accessories_result = homebridge::get_all_accessories(token, uri.to_string()).await;
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
