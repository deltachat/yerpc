use crate::{Error, Message, Params, Request, Response, RpcHandler, Version};
use async_mutex::Mutex;
use futures::channel::oneshot;
use serde::Serialize;
use std::{collections::HashMap, sync::Arc};

pub struct MessageHandle<T> {
    requests: RpcHandle,
    session: T,
}
impl<T: RpcHandler> MessageHandle<T> {
    pub fn new(requests: RpcHandle, session: T) -> Self {
        Self { requests, session }
    }
    pub async fn handle_message(&self, input: &str) {
        let message: Message = match serde_json::from_str(input) {
            Ok(message) => message,
            Err(err) => {
                let _ = self
                    .requests
                    .tx(Message::Response(Response::error(
                        None,
                        Error::new(Error::PARSE_ERROR, err),
                    )))
                    .await;
                return;
            }
        };

        match message {
            Message::Request(request) => {
                let params = request.params.map(Params::into_value).unwrap_or_default();
                let response = match request.id {
                    None | Some(0) => {
                        match self.session.on_notification(request.method, params).await {
                            Ok(()) => None,
                            Err(err) => Some(Response::error(request.id, err)),
                        }
                    }
                    Some(id) => match self.session.on_request(request.method, params).await {
                        Ok(payload) => Some(Response::success(id, payload)),
                        Err(err) => Some(Response::error(Some(id), err)),
                    },
                };
                if let Some(response) = response {
                    let _ = self.requests.tx(Message::Response(response)).await;
                }
            }
            Message::Response(response) => {
                self.requests.on_response(response).await;
            }
        };
    }
}

#[derive(Clone)]
pub struct RpcHandle {
    inner: Arc<Mutex<RequestMap>>,
    tx: async_channel::Sender<Message>,
}

fn downcast_params<T: Serialize>(params: Option<T>) -> Result<Option<Params>, Error> {
    if let Some(params) = params {
        let params = serde_json::to_value(params).map_err(|_| Error::bad_request())?;
        match params {
            serde_json::Value::Array(params) => Ok(Some(Params::Positional(params))),
            serde_json::Value::Object(params) => Ok(Some(Params::Structured(params))),
            _ => Err(Error::bad_request()),
        }
    } else {
        Ok(None)
    }
}

impl RpcHandle {
    pub fn new() -> (Self, async_channel::Receiver<Message>) {
        let (tx, rx) = async_channel::bounded(10);
        let inner = RequestMap::new();
        let inner = Arc::new(Mutex::new(inner));
        let this = Self { inner, tx };
        (this, rx)
    }
    pub async fn request(
        &self,
        method: impl ToString,
        params: Option<impl Serialize>,
    ) -> Result<serde_json::Value, Error> {
        let method = method.to_string();
        let params = downcast_params(params)?;
        let (message, rx) = self.inner.lock().await.request(method, params);
        self.tx(message).await?;
        // Wait for response to arrive.
        // TODO: Better error.
        let res = rx.await.map_err(|_| Error::bad_response())?;
        match (res.result, res.error) {
            (Some(result), None) => Ok(result),
            (None, Some(error)) => Err(error),
            // TODO: better error.
            _ => Err(Error::bad_response()),
        }
    }

    pub async fn notify(
        &self,
        method: impl ToString,
        params: Option<impl Serialize>,
    ) -> Result<(), Error> {
        let method = method.to_string();
        let params = downcast_params(params)?;
        let request = Request {
            jsonrpc: Version::V2,
            method: method.to_string(),
            params,
            id: None,
        };
        let message = Message::Request(request);
        self.tx(message).await?;
        Ok(())
    }

    pub(crate) async fn tx(&self, message: Message) -> Result<(), Error> {
        self.tx
            .send(message)
            .await
            .map_err(|_| Error::remote_disconnected())?;
        Ok(())
    }

    pub async fn on_response(&self, response: Response) {
        self.inner.lock().await.on_response(response)
    }
}

#[derive(Default)]
pub struct RequestMap {
    next_request_id: usize,
    requests: HashMap<usize, oneshot::Sender<Response>>,
    // tx: async_channel::Sender<Message>,
}

impl RequestMap {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn request(
        &mut self,
        method: String,
        params: Option<Params>,
    ) -> (Message, oneshot::Receiver<Response>) {
        let request_id = self.next_request_id;
        self.next_request_id += 1;
        let (tx, rx) = oneshot::channel();
        self.requests.insert(request_id, tx);
        let request = Request {
            jsonrpc: Version::V2,
            method,
            params,
            id: Some(request_id as u32),
        };
        let message = Message::Request(request);
        (message, rx)
    }
    pub fn on_response(&mut self, response: Response) {
        if let Some(id) = &response.id {
            let tx = self.requests.remove(&(*id as usize));
            if let Some(tx) = tx {
                let _ = tx.send(response);
            }
        }
    }
}
