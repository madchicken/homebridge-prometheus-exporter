use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use hyper::{Body, Client, Request};


pub mod session {
    use std::time::{Duration, SystemTime};
    use serde::{Serialize, Deserialize};
    use crate::homebridge::login;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Token {
        pub access_token: String,
        pub token_type: String,
        pub expires_in: u64,
    }

    pub struct Session {
        token: Option<Box<Token>>,
        username: String,
        password: String,
        uri: String,
        pub created_at: SystemTime
    }

    impl Session {

        pub fn new(username: String, password: String, uri: String) -> Session {
            Session {
                token: None,
                username: username.to_string(),
                password: password.to_string(),
                uri: uri.to_string(),
                created_at: SystemTime::now(),
            }
        }

        pub fn is_valid(&self) -> bool {
            if self.token.is_some() {
                SystemTime::now().duration_since(self.created_at).unwrap()
                    .ge(&Duration::from_secs(self.token.as_ref().unwrap().expires_in))
            } else {
                false
            }
        }

        pub async fn get_token(&mut self) -> Box<Token> {
            if !self.is_valid() {
                let result = login(self.username.to_string(), self.password.to_string(), self.uri.to_string()).await.unwrap_or_else(|e| panic!("{}", e));
                let _ = self.token.insert(Box::new(result));
            }
            self.token.clone().unwrap()
        }

    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServiceCharacteristics {
    pub aid: u16,
    pub iid: u16,
    pub uuid: String,
    #[serde(rename(deserialize = "type"))]
    pub type_: String,
    #[serde(rename(deserialize = "serviceType"))]
    pub service_type: String,
    #[serde(rename(deserialize = "serviceName"))]
    pub service_name: String,
    pub description: String,
    pub value: Value,
    pub format: String,
    pub perms: Vec<String>,
    #[serde(rename(deserialize = "canRead"))]
    pub can_read: bool,
    #[serde(rename(deserialize = "canWrite"))]
    pub can_write: bool,
    pub ev: bool
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Instance {
    pub name: String,
    pub username: String,
    #[serde(rename(deserialize = "ipAddress"))]
    pub ip_address: String,
    pub port: u16,
    pub services: Vec<Value>,
    #[serde(rename(deserialize = "connectionFailedCount"))]
    pub connection_failed_count: u16
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Accessory {
    pub aid: u32,
    pub iid: u32,
    pub uuid: String,
    #[serde(rename(deserialize = "type"))]
    pub accessory_type: String,
    #[serde(rename(deserialize = "humanType"))]
    pub human_type: String,
    #[serde(rename(deserialize = "serviceName"))]
    pub service_name: String,
    #[serde(rename(deserialize = "serviceCharacteristics"))]
    pub service_characteristics: Vec<ServiceCharacteristics>,
    #[serde(rename(deserialize = "accessoryInformation"))]
    pub accessory_information: Value,
    pub instance: Instance,
    pub values: Value,
    #[serde(rename(deserialize = "uniqueId"))]
    pub unique_id: String,
}

pub async fn login(username: String, password: String, uri: String) -> Result<session::Token, String> {
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
    let token: session::Token = serde_json::from_str(&body).unwrap();
    println!("Fetched token. New token is valid for {} seconds", token.expires_in);
    return Ok(token)
}

pub async fn get_all_accessories(token: &session::Token, uri: String) -> Result<Vec<Accessory>, String> {
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
    let accessories: Vec<Accessory> = serde_json::from_str(&body).unwrap();
    println!("Fetched {} accessories", accessories.len());
    return Ok(accessories)
}
