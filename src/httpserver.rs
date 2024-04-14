use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::Router;
use axum::routing::{get, post};
use inflector::cases::snakecase::to_snake_case;
use prometheus_client::encoding::text::encode;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use tokio::signal;
use tokio::sync::Mutex;
use tower_http::compression::CompressionLayer;

use crate::{Config, homebridge};
use crate::homebridge::session::Session;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct AuthorizationKeys {
    keys: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct SuccessResponse {
    result: String,
}

fn load_keys(keyfile_path: String) -> AuthorizationKeys {
    let f = std::fs::File::open(keyfile_path);
    match f {
        Ok(file) => {
            let keys: AuthorizationKeys = serde_yaml::from_reader(file)
                .expect("Could not read values from authorization key file.");
            debug!("{:?}", keys);
            keys
        }
        Err(e) => {
            warn!("Could not open authorization key file. {}", e.to_string());
            warn!("Using an empty key set, authorization won't be available.");
            AuthorizationKeys { keys: vec![] }
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    session: Arc<Mutex<Session>>,
    config: Arc<Config>,
    keys: Arc<AuthorizationKeys>,
}

/// Start a HTTP server to report metrics.
pub async fn start_metrics_server(config: Config) {
    debug!("Creating session");
    let port = config.port;
    let password = config.password.clone();
    let username = config.username.clone();
    let uri = config.uri.clone();
    let keys: AuthorizationKeys = load_keys(config.keyfile.clone());
    let shared_config = Arc::new(config);
    let shared_keys = Arc::new(keys);
    let session = Arc::new(Mutex::new(Session::new(username, password, uri)));
    debug!("Session created {:?}", session);

    let state = AppState {
        session,
        config: shared_config,
        keys: shared_keys,
    };

    let routes = Router::new()
        .route("/ping", get(ping))
        .route("/metrics", get(metrics))
        .route("/restart", post(restart))
        .layer(CompressionLayer::new())
        .with_state(state);

    let address = format!("0.0.0.0:{}", port);
    let addr: SocketAddr = address.parse().unwrap();
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, routes)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("signal received, starting graceful shutdown");
}

fn check_bearer_token(headers: &HeaderMap, keys: &[String]) -> bool {
    if headers.contains_key("Authorization") {
        let bearer = headers.get("Authorization").unwrap().to_str().unwrap();
        let parts: Vec<_> = bearer.split(' ').collect();
        if parts[0].eq("Bearer") {
            let req_key = parts[1];
            let index = keys.iter().position(|key| key.eq(req_key));
            return index.is_some();
        }
        return false;
    }
    false
}

async fn restart(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    match check_bearer_token(&headers, &state.keys.keys) {
        true => {
            let token_res = state.session.lock().await.get_token().await;
            match token_res {
                Ok(token) => {
                    let result = homebridge::restart(token, state.config.uri.clone()).await;
                    match result {
                        Ok(_b) => (
                            StatusCode::OK,
                            [(header::CONTENT_TYPE, "application/json")],
                            serde_json::to_string(&SuccessResponse {
                                result: "done".to_string(),
                            })
                            .unwrap(),
                        )
                            .into_response(),
                        Err(e) => (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            [(header::CONTENT_TYPE, "application/json")],
                            serde_json::to_string(&ErrorResponse { error: e }).unwrap(),
                        )
                            .into_response(),
                    }
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [(header::CONTENT_TYPE, "application/json")],
                    serde_json::to_string(&ErrorResponse { error: e }).unwrap(),
                )
                    .into_response(),
            }
        }
        false => (
            StatusCode::UNAUTHORIZED,
            [(header::CONTENT_TYPE, "application/json")],
            serde_json::to_string(&ErrorResponse {
                error: "Unauthorized request, please provide a valid token.".to_string(),
            })
            .unwrap(),
        )
            .into_response(),
    }
}

async fn ping(State(_state): State<AppState>) -> impl IntoResponse {
    (StatusCode::OK, String::from("PONG"))
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    let mut guard = state.session.lock().await;
    let token_result = guard.get_token().await;
    match token_result {
        Ok(token) => {
            let prefix = state
                .config
                .prefix
                .clone()
                .unwrap_or("homebridge".to_string());
            let uri = state.config.uri.clone();
            let result = build_registry(token, uri, prefix).await;
            match result {
                Ok(registry) => {
                    let mut buffer = String::new();
                    encode(&mut buffer, &registry).unwrap();
                    (StatusCode::OK, buffer)
                }
                Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            }
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e),
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
                    if !service.format.eq_ignore_ascii_case("string") {
                        // ignore string service types
                        let metric =
                            Family::<Vec<(String, String)>, Gauge<f64, AtomicU64>>::default();
                        let metric_name = format!(
                            "{}_{}",
                            to_snake_case(&service.service_type.to_string()),
                            to_snake_case(&service.type_.to_string())
                        );
                        let value_as_float = service.value.as_f64().unwrap_or(0.0);
                        registry.register(
                            metric_name.to_string(),
                            service.description,
                            metric.clone(),
                        );

                        metric
                            .get_or_create(&vec![(
                                "name".to_owned(),
                                to_snake_case(&service.service_name.to_string()).to_owned(),
                            )])
                            .set(value_as_float);
                    }
                }
            }
            Ok(registry)
        }
        Err(e) => {
            error!("{}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderName};
    use reqwest::header::HeaderValue;

    use crate::httpserver::check_bearer_token;

    #[test]
    fn check_bearer_token_find_the_right_token() {
        let keys = vec![String::from("foo"), String::from("bar")];
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str("Bearer bar").unwrap(),
        );

        assert!(check_bearer_token(&headers, &keys));
    }

    #[test]
    fn check_bearer_token_fails_with_wrong_token() {
        let keys = vec![String::from("foo"), String::from("bar")];
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str("Bearer zoo").unwrap(),
        );

        assert!(!check_bearer_token(&headers, &keys));
    }

    #[test]
    fn check_bearer_token_fails_with_empty_keys() {
        let keys = vec![];
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("authorization"),
            HeaderValue::from_str("Bearer zoo").unwrap(),
        );

        assert!(!check_bearer_token(&headers, &keys));
    }
}
