use std::sync::Mutex;
use inflector::cases::snakecase::to_snake_case;
use prometheus_client::{encoding::text::encode, registry::Registry};
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use actix_web::{web, App, HttpServer, HttpResponse, web::Data, HttpRequest};
use actix_web::http::header::HeaderMap;
use actix_web::http::StatusCode;

use crate::{Config, homebridge};
use crate::homebridge::session::{Session};

use serde::{Deserialize, Serialize};
use serde_yaml::{self};

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AuthorizationKeys {
    keys: Vec<String>,
}

fn load_keys(keyfile_path: String) -> AuthorizationKeys {
    let f = std::fs::File::open(keyfile_path);
    match f {
        Ok(file) => {
            let keys: AuthorizationKeys = serde_yaml::from_reader(file).expect("Could not read values from authorization key file.");
            debug!("{:?}", keys);
            keys
        }
        Err(e) => {
            warn!("Could not open authorization key file. {}", e.to_string());
            warn!("Using an empty key set, authorization won't be available.");
            AuthorizationKeys {
                keys: vec![]
            }
        }
    }
}

/// Start a HTTP server to report metrics.
pub async fn start_metrics_server(config: Config) -> std::io::Result<()> {
    debug!("Creating session");
    let port = config.port;
    let password = config.password.clone();
    let username = config.username.clone();
    let uri = config.uri.clone();
    let keys: AuthorizationKeys = load_keys(config.keyfile.clone());
    let shared_config = Data::new(config);
    let shared_keys = Data::new(keys);
    let session: Session = Session::new(username, password, uri);
    debug!("Session created {:?}", session);
    let shared_session = Data::new(Mutex::new(session));

    let bind_address = "0.0.0.0";
    info!("Serving /metrics at http://{}:{}", bind_address, port);
    HttpServer::new(move || {
        App::new()
            .app_data(shared_session.clone())
            .app_data(shared_config.clone())
            .app_data(shared_keys.clone())
            .service(web::resource("/metrics").route(web::get().to(metrics_get)))
            .service(web::resource("/restart").route(web::post().to(restart)))
    })
        .bind((bind_address, port))?
        .run()
        .await
}

fn check_bearer_token(headers: &HeaderMap, keys: &Vec<String>) -> bool {
    if headers.contains_key("Authorization") {
        let bearer = headers.get("Authorization").unwrap().to_str().unwrap();
        let parts: Vec<_> = bearer.split(' ').collect();
        if parts[0].eq("Bearer") {
            let req_key = parts[1];
            let index = keys
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
    match check_bearer_token(req.headers(), &keys.keys) {
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
            error!("{}", e);
            Err(format!("{}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::http::header::{HeaderMap, HeaderName};
    use reqwest::header::HeaderValue;
    use crate::httpserver::check_bearer_token;

    #[test]
    fn check_bearer_token_find_the_right_token() {
        let keys = vec![String::from("foo"), String::from("bar")];
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("authorization"), HeaderValue::from_str("Bearer bar").unwrap());

        assert_eq!(check_bearer_token(&headers, &keys), true);
    }

    #[test]
    fn check_bearer_token_fails_with_wrong_token() {
        let keys = vec![String::from("foo"), String::from("bar")];
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("authorization"), HeaderValue::from_str("Bearer zoo").unwrap());

        assert_eq!(check_bearer_token(&headers, &keys), false);
    }

    #[test]
    fn check_bearer_token_fails_with_empty_keys() {
        let keys = vec![];
        let mut headers = HeaderMap::new();
        headers.insert(HeaderName::from_static("authorization"), HeaderValue::from_str("Bearer zoo").unwrap());

        assert_eq!(check_bearer_token(&headers, &keys), false);
    }
}