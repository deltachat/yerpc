use std::collections::HashMap;

pub use async_trait::async_trait;
use serde::{Deserialize, Serialize};
pub use typescript_type_def::TypeDef;

pub use jsonrpc_derive::rpc;
mod version;
pub use version::Version;
pub mod typescript;

pub type Id = u32;
pub type Result<T> = std::result::Result<T, Error>;

/// Only used for generated TS bindings
#[derive(Serialize, Deserialize, Debug, TypeDef)]
#[serde(untagged)]
pub enum RpcResult<T: TypeDef> {
    Ok(T),
    Err(Error),
}

#[derive(Serialize, Deserialize, Debug, TypeDef)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
}

#[derive(Serialize, Deserialize, Debug, TypeDef)]
#[serde(untagged)]
pub enum Params {
    Positional(Vec<serde_json::Value>),
    Structured(serde_json::Map<String, serde_json::Value>),
}

impl Params {
    pub fn into_value(self) -> serde_json::Value {
        match self {
            Params::Positional(list) => serde_json::Value::Array(list),
            Params::Structured(object) => serde_json::Value::Object(object),
        }
    }
}

impl From<Params> for serde_json::Value {
    fn from(params: Params) -> Self {
        params.into_value()
    }
}

impl TryFrom<serde_json::Value> for Params {
    type Error = Error;
    fn try_from(value: serde_json::Value) -> std::result::Result<Self, Self::Error> {
        match value {
            serde_json::Value::Object(object) => Ok(Params::Structured(object)),
            serde_json::Value::Array(list) => Ok(Params::Positional(list)),
            _ => Err(Error::invalid_params()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, TypeDef)]
pub struct Request {
    pub jsonrpc: Version,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Params>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

#[derive(Serialize, Deserialize, Debug, TypeDef)]
pub struct Response {
    pub jsonrpc: Version,
    pub id: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}
impl Response {
    pub fn error(id: Option<Id>, error: Error) -> Self {
        Self {
            jsonrpc: Version::V2,
            id,
            error: Some(error),
            result: None,
        }
    }
    pub fn success(id: Id, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: Version::V2,
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, TypeDef)]
pub struct Error {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "JSON-RPC error: {} (code {})", self.message, self.code)
    }
}
impl Error {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    pub fn new(code: i32, message: impl ToString) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
        }
    }

    pub fn with_data(code: i32, message: String, data: Option<serde_json::Value>) -> Self {
        Self {
            code,
            message,
            data,
        }
    }

    pub fn invalid_params() -> Self {
        Self::new(Error::INVALID_PARAMS, "Params has to be an object or array")
    }

    pub fn method_not_found() -> Self {
        Self::new(Error::METHOD_NOT_FOUND, "Method not found")
    }

    pub fn invalid_args_len(n: usize) -> Self {
        Self::new(
            Error::INVALID_PARAMS,
            format!("This method takes an array of {} arguments", n),
        )
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self {
            code: Error::PARSE_ERROR,
            message: format!("{}", error),
            data: None,
        }
    }
}

#[cfg(feature = "anyhow")]
impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self {
            code: Error::INTERNAL_ERROR,
            message: "Internal server error".to_string(),
            data: None,
        }
    }
}

#[async_trait]
pub trait RpcHandler: Sync + Send + 'static {
    async fn on_notification(&self, _method: String, _params: serde_json::Value) -> Result<()> {
        Ok(())
    }
    async fn on_request(
        &self,
        _method: String,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        Err(Error::new(Error::METHOD_NOT_FOUND, "Method not found"))
    }
}

pub async fn handle_message<T: RpcHandler>(session: &T, input: &str) -> Option<String> {
    let message: Message = match serde_json::from_str(input) {
        Ok(message) => message,
        Err(err) => {
            return Some(
                serde_json::to_string(&Response::error(None, Error::new(Error::PARSE_ERROR, err)))
                    .unwrap(),
            )
        }
    };

    let response = match message {
        Message::Request(request) => {
            let params = request.params.map(Params::into_value).unwrap_or_default();
            match request.id {
                None | Some(0) => match session.on_notification(request.method, params).await {
                    Ok(()) => None,
                    Err(err) => Some(Response::error(request.id, err)),
                },
                Some(id) => match session.on_request(request.method, params).await {
                    Ok(payload) => Some(Response::success(id, payload)),
                    Err(err) => Some(Response::error(Some(id), err)),
                },
            }
        }
        Message::Response(_response) => Some(Response::error(
            None,
            Error::new(
                Error::INVALID_REQUEST,
                "Receiving responses is unsupported.",
            ),
        )),
    };

    match response {
        None => None,
        Some(response) => match serde_json::to_string(&response) {
            Ok(string) => Some(string),
            Err(err) => {
                log::error!("Failed to serialize response {}", err);
                Some(
                    serde_json::to_string(&Response::error(
                        response.id,
                        Error::new(Error::INTERNAL_ERROR, "Failed to serialize response"),
                    ))
                    .expect("Serialization failure"),
                )
            }
        },
    }
}
