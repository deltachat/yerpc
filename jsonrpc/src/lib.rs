use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub use jsonrpc_derive::rpc;

pub type Id = u32;

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum Message {
    Request(Request),
    Response(Response),
}

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    jsonrpc: String,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Id>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    jsonrpc: String,
    id: Option<Id>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Error>,
}
impl Response {
    pub fn error(id: Option<Id>, error: Error) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            error: Some(error),
            result: None,
        }
    }
    pub fn success(id: Id, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id: Some(id),
            result: Some(result),
            error: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
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

    pub fn method_not_found() -> Self {
        Self::new(Error::METHOD_NOT_FOUND, "Method not found")
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

#[async_trait]
pub trait RpcHandler: Sync + Send + 'static {
    async fn on_notification(
        &self,
        method: String,
        params: serde_json::Value,
    ) -> Result<(), Error> {
        Ok(())
    }
    async fn on_request(
        &self,
        method: String,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
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
            // match request.params {
            //     Some(serde_json::Value::Object(_)) | Some(serde_json::Value::Array(_)) | None => {},
            //     _ =>
            // }
            match request.id {
                None | Some(0) => match session
                    .on_notification(request.method, request.params.unwrap_or_default())
                    .await
                {
                    Ok(()) => None,
                    Err(err) => Some(Response::error(request.id, err)),
                },
                Some(id) => match session
                    .on_request(request.method, request.params.unwrap_or_default())
                    .await
                {
                    Ok(payload) => Some(Response::success(id, payload)),
                    Err(err) => Some(Response::error(Some(id), err)),
                },
            }
        }
        Message::Response(response) => Some(Response::error(
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

// #[derive(Serialize, Deserialize, Debug)]
// #[serde(untagged)]
// enum Params {
//     List(Vec<serde_json::Value>),
//     Object(HashMap<String, serde_json::Value>),
// }
