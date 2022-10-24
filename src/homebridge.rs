use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use hyper::{Body, Client, Request};

#[derive(Serialize, Deserialize, Debug)]
pub struct Token {
    pub access_token: String,
    token_type: String,
    pub expires_in: i32,
}

pub async fn login(username: String, password: String, uri: String) -> Result<Token, String> {
    let login = json!({
        "username": username,
        "password": password,
        "otp": "123"
    });

    let request: Request<Body> = Request::post(format!("{}/api/auth/login", uri))
        .header("content-type", "application/json")
        .body(Body::from(login.to_string())).unwrap();

    let client = Client::new();
    let response = client.request(request).await.unwrap();
    if !response.status().is_success() {
        return Err(format!("Error while fetching token. Error code: {}",response.status()))
    }

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body = String::from_utf8(body_bytes.to_vec()).unwrap();
    let token: Token = serde_json::from_str(&body).unwrap();
    return Ok(token)
}

pub async fn get_all_accessories(token: &Token, uri: String) -> Result<Vec<Value>, String> {
    let client = Client::new();
    let request = Request::get(format!("{}/api/accessories", uri))
        .header("content-type", "application/json")
        .header("Authorization", format!("Bearer {}", token.access_token))
        .body(Body::empty())
        .unwrap();

    let response = client.request(request).await.unwrap();
    if !response.status().is_success() {
        return Err(format!("Error while fetching token. Error code: {}",response.status()))
    }

    let body_bytes = hyper::body::to_bytes(response.into_body()).await.unwrap();
    let body = String::from_utf8(body_bytes.to_vec()).unwrap();
    let accessories: Vec<Value> = serde_json::from_str(&body).unwrap();
    return Ok(accessories)
}
