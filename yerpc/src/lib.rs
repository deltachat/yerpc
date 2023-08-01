#![warn(clippy::wildcard_imports)]

pub use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use yerpc_derive::rpc;

#[cfg(feature = "openrpc")]
pub mod openrpc;
mod requests;
pub mod typescript;
mod version;

#[cfg(feature = "openrpc")]
pub use openrpc::JsonSchema;
pub use requests::{OutReceiver, RpcClient, RpcSession, RpcSessionSink};
pub use typescript::TypeDef;
pub use version::Version;

mod integrations;
pub use integrations::*;

#[async_trait]
pub trait RpcServer: Sync + Send + 'static {
    /// Returns OpenRPC specification as a string.
    #[cfg(feature = "openrpc")]
    fn openrpc_specification() -> Result<String> {
        Ok(String::new())
    }

    async fn handle_notification(&self, _method: String, _params: serde_json::Value) -> Result<()> {
        Ok(())
    }
    async fn handle_request(
        &self,
        _method: String,
        _params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        Err(Error::new(
            Error::METHOD_NOT_FOUND,
            "Method not found".to_string(),
        ))
    }
}

impl RpcServer for () {}

/// Request identifier as found in Request and Response objects.
#[derive(Serialize, Deserialize, Debug, TypeDef, Eq, Hash, PartialEq, Clone)]
#[serde(untagged)]
pub enum Id {
    Number(u32),
    String(String),
}

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

/// Request object.
#[derive(Serialize, Deserialize, Debug, TypeDef)]
pub struct Request {
    /// JSON-RPC protocol version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jsonrpc: Option<Version>, // JSON-RPC 1.0 has no jsonrpc field

    /// Name of the method to be invoked.
    pub method: String,

    /// Method parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Params>,

    /// Request identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
}

/// Response object.
#[derive(Serialize, Deserialize, Debug, TypeDef)]
pub struct Response {
    /// JSON-RPC protocol version.
    pub jsonrpc: Version,

    /// Request identifier.
    pub id: Option<Id>,

    /// Return value of the method.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,

    /// Error occured during the method invocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Error>,
}
impl Response {
    /// Creates a new Response object indicating an error.
    pub fn error(id: Option<Id>, error: Error) -> Self {
        Self {
            jsonrpc: Version::V2,
            id,
            error: Some(error),
            result: None,
        }
    }
    /// Creates a new Response object indicating a success.
    pub fn success(id: Id, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: Version::V2,
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }
}

/// Error object returned in response to a failed RPC call.
#[derive(Serialize, Deserialize, Debug, TypeDef)]
#[cfg_attr(feature = "openrpc", derive(JsonSchema))]
pub struct Error {
    /// Error code indicating the error type.
    pub code: i32,

    /// Short error description.
    pub message: String,

    /// Additional information about the error.
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

    pub const BAD_REQUEST: i32 = -32000;
    pub const BAD_RESPONSE: i32 = -32001;
    pub const REMOTE_DISCONNECTED: i32 = -32002;

    /// Creates a new error object.
    pub fn new(code: i32, message: String) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    /// Creates a new error object with additional information.
    pub fn with_data(code: i32, message: String, data: Option<serde_json::Value>) -> Self {
        Self {
            code,
            message,
            data,
        }
    }

    /// Creates a new error object indicating invalid method parameters.
    pub fn invalid_params() -> Self {
        Self::new(
            Error::INVALID_PARAMS,
            "Params has to be an object or array".to_string(),
        )
    }

    /// Creates a new error object indicating that the method does not exist.
    pub fn method_not_found() -> Self {
        Self::new(Error::METHOD_NOT_FOUND, "Method not found".to_string())
    }

    pub fn invalid_args_len(n: usize) -> Self {
        Self::new(
            Error::INVALID_PARAMS,
            format!("This method takes an array of {n} arguments"),
        )
    }

    pub fn bad_response() -> Self {
        Self::new(
            Error::BAD_RESPONSE,
            "Error while processing a response".to_string(),
        )
    }
    pub fn bad_request() -> Self {
        Self::new(
            Error::BAD_REQUEST,
            "Error while serializing a request".to_string(),
        )
    }
    pub fn remote_disconnected() -> Self {
        Self::new(
            Error::REMOTE_DISCONNECTED,
            "Remote disconnected".to_string(),
        )
    }

    pub fn is_disconnnected(&self) -> bool {
        self.code == Error::REMOTE_DISCONNECTED
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self {
            code: Error::PARSE_ERROR,
            message: format!("{error}"),
            data: None,
        }
    }
}

#[cfg(feature = "anyhow")]
#[cfg(feature = "anyhow_expose")]
impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self {
            code: -1,
            message: format!("{:?}", error),
            data: None,
        }
    }
}

#[cfg(feature = "anyhow")]
#[cfg(not(feature = "anyhow_expose"))]
impl From<anyhow::Error> for Error {
    fn from(_error: anyhow::Error) -> Self {
        Self {
            code: Error::INTERNAL_ERROR,
            message: "Internal server error".to_string(),
            data: None,
        }
    }
}
