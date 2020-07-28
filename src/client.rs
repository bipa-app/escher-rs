use chrono::{DateTime, Utc};
use failure::Fail;
use serde::de::Deserializer;
use serde::Deserialize;
use serde_json::json;
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
    pub expires_in: i32,
    pub token_type: String,
    pub refresh_token: String,
    pub id_token: String,
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
    pub created_at_server: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct AcceptQuote {
    pub success: bool,
    pub quote_id: String,
    pub order: Order,
}

#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Escher Client - API Error {}", _0)]
    NetworkingError(#[cause] reqwest::Error),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Error {
        Self::NetworkingError(error)
    }
}

type EscherResult<Data> = Result<Data, Error>;

#[derive(Deserialize, Debug)]
pub enum Side {
    Buy,
    Sell,
}

impl ToString for Side {
    fn to_string(&self) -> String {
        match self {
            Side::Buy => "buy".to_string(),
            Side::Sell => "sell".to_string(),
        }
    }
}

impl Client {
    pub fn init(url: String) -> Self {
        Self { url }
    }

    pub fn sign_in(&self, email: String, password: String) -> EscherResult<AuthResponse> {
        reqwest::blocking::Client::new()
            .post(&format!("{}/sign-in", self.url))
            .json(&json!({"email": email, "password": password}))
            .send()?
            .json::<AuthResponse>()
            .map_err(Into::into)
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
            "side": side.to_string()
        });

        reqwest::blocking::Client::new()
            .post(&format!("{}/quotes", self.url))
            .json(&params)
            .header("i2-ACCESS-KEY", access_token)
            .send()?
            .json::<Quote>()
            .map_err(Into::into)
    }

    pub fn accept_quote(
        &self,
        access_token: String,
        quote_id: String,
        quantity: Option<f32>,
    ) -> EscherResult<AcceptQuote> {
        reqwest::blocking::Client::new()
            .post(&format!("{}/quotes/accept", self.url))
            .json(&json!({"quote_id": quote_id, "quantity" : quantity}))
            .header("i2-ACCESS-KEY", access_token)
            .send()?
            .json::<AcceptQuote>()
            .map_err(Into::into)
    }
}
