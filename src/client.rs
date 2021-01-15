use chrono::{DateTime, Utc};
use failure::Fail;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use std::fmt::Display;
use std::str;
use std::str::FromStr;

pub struct Client {
    pub url: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AuthResult {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct AuthResponse {
    pub authentication_result: AuthResult,
}

fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(serde::de::Error::custom)
}

#[derive(Deserialize, Debug)]
pub struct Quote {
    pub quote_id: String,
    pub product_id: String,
    pub base_currency: String,
    #[serde(deserialize_with = "from_str")]
    pub price: f32,
    #[serde(deserialize_with = "from_str")]
    pub base_currency_size: f32,
    #[serde(deserialize_with = "from_str")]
    pub quote_currency_size: f32,
    pub side: Side,
    pub created_at: DateTime<Utc>,
    pub expiry: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct Order {
    pub id: String,
    pub product_id: String,
    pub order_type: String,
    pub order_status: String,
    pub time_in_force: String,
    #[serde(deserialize_with = "from_str")]
    pub fill_price: f32,
    #[serde(deserialize_with = "from_str")]
    pub fill_qty: f32,
    #[serde(deserialize_with = "from_str")]
    pub price: f32,
    #[serde(deserialize_with = "from_str")]
    pub order_size: f32,
    pub client_side: Side,
    pub status: String,
    #[serde(deserialize_with = "from_str")]
    pub executed_value: f32,
}

#[derive(Deserialize, Debug)]
pub struct AcceptQuote {
    pub success: bool,
    pub quote_id: String,
    pub order: Order,
}

#[derive(Deserialize, Debug, Fail)]
pub struct EscherError {
    pub success: bool,
    pub message: String,
}

impl fmt::Display for EscherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "JSON Decoding Error {}", _0)]
    DecodingError(serde_json::Error),
    #[fail(display = "NetworkingError {}", _0)]
    NetworkingError(#[cause] reqwest::Error),
    #[fail(display = "EscherError - {}", _0)]
    HandledError(EscherError),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Error {
        Self::NetworkingError(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Error {
        Self::DecodingError(error)
    }
}
type EscherResult<Data> = Result<Data, Error>;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Buy,
    Sell,
}

impl Client {
    pub fn init(url: String) -> Self {
        Self { url }
    }

    pub fn sign_in(&self, email: String, password: String) -> EscherResult<AuthResponse> {
        let resp = reqwest::blocking::Client::new()
            .post(&format!("{}/sign-in", self.url))
            .json(&json!({"email": email, "password": password}))
            .send()?;

        let json = resp.json::<serde_json::Value>();

        match json {
            Ok(auth) => serde_json::from_value::<AuthResponse>(auth.clone()).map_err(|_| {
                match serde_json::from_value::<EscherError>(auth) {
                    Ok(err) => Error::HandledError(err),
                    Err(err) => err.into(),
                }
            }),
            Err(err) => Err(err.into()),
        }
    }

    pub fn refresh_token(&self, token: String, email: String) -> EscherResult<AuthResponse> {
        let resp = reqwest::blocking::Client::new()
            .post(&format!("{}/sign-in", self.url))
            .json(&json!({ "refreshToken": token, "email": email }))
            .send()?;

        let json = resp.json::<serde_json::Value>();
        match json {
            Ok(auth) => serde_json::from_value::<AuthResponse>(auth.clone()).map_err(|_| {
                match serde_json::from_value::<EscherError>(auth) {
                    Ok(err) => Error::HandledError(err),
                    Err(err) => err.into(),
                }
            }),
            Err(err) => Err(err.into()),
        }
    }

    pub fn quote(
        &self,
        access_token: String,
        product_id: String,
        base_currency_size: String,
        side: Side,
    ) -> EscherResult<Quote> {
        let params = json!({
            "product_id": product_id,
            "base_currency_size": base_currency_size,
            "side": side
        });

        let resp = reqwest::blocking::Client::new()
            .post(&format!("{}/quotes", self.url))
            .json(&params)
            .header("i2-ACCESS-KEY", access_token)
            .send()?;

        let quote = resp.json::<serde_json::Value>()?;

        serde_json::from_value::<Quote>(quote.clone()).map_err(|_| {
            match serde_json::from_value::<EscherError>(quote) {
                Ok(err) => Error::HandledError(err),
                Err(err) => err.into(),
            }
        })
    }

    pub fn accept_quote(
        &self,
        access_token: String,
        quote_id: String,
        quantity: Option<f32>,
    ) -> EscherResult<AcceptQuote> {
        let resp = reqwest::blocking::Client::new()
            .post(&format!("{}/quotes/accept", self.url))
            .json(&json!({"quote_id": quote_id, "quantity" : quantity}))
            .header("i2-ACCESS-KEY", access_token)
            .send()?;

        let quote = resp.json::<serde_json::Value>()?;

        serde_json::from_value::<AcceptQuote>(quote.clone()).map_err(|_| {
            match serde_json::from_value::<EscherError>(quote) {
                Ok(err) => Error::HandledError(err),
                Err(err) => err.into(),
            }
        })
    }
}
