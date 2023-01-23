use async_mutex::Mutex;
use futures::channel::oneshot;
use futures_util::{Future, Sink};
use serde::Serialize;
use std::io;
use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use crate::{Error, Id, Message, Params, Request, Response, RpcServer, Version};

// pub fn create_session(server_impl: impl RpcServer) -> (RpcSession<T>,

pub struct RpcSession<T> {
    client: RpcClient,
    server: T,
}

impl<T: Clone> Clone for RpcSession<T> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            server: self.server.clone(),
        }
    }
}

impl<T: RpcServer> RpcSession<T> {
    pub fn create(server: T) -> (Self, async_channel::Receiver<Message>) {
        let (client, receiver) = RpcClient::new();
        (Self::new(client, server), receiver)
    }

    pub fn new(client: RpcClient, server: T) -> Self {
        Self { client, server }
    }

    pub fn client(&self) -> &RpcClient {
        &self.client
    }

    pub fn into_sink(self) -> RpcSessionSink<T> {
        RpcSessionSink::Idle(Some(self))
    }

    pub async fn handle_incoming(&self, input: &str) {
        let message: Message = match serde_json::from_str(input) {
            Ok(message) => message,
            Err(err) => {
                let _ = self
                    .client
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
                    None | Some(Id::Number(0)) => {
                        match self
                            .server
                            .handle_notification(request.method, params)
                            .await
                        {
                            Ok(()) => None,
                            Err(err) => Some(Response::error(request.id, err)),
                        }
                    }
                    Some(id) => match self.server.handle_request(request.method, params).await {
                        Ok(payload) => Some(Response::success(id, payload)),
                        Err(err) => Some(Response::error(Some(id), err)),
                    },
                };
                if let Some(response) = response {
                    let _ = self.client.tx(Message::Response(response)).await;
                }
            }
            Message::Response(response) => {
                self.client.handle_response(response).await;
            }
        };
    }
}

#[derive(Clone)]
pub struct RpcClient {
    inner: Arc<Mutex<PendingRequests>>,
    tx: async_channel::Sender<Message>,
}

pub type OutReceiver = async_channel::Receiver<Message>;

impl RpcClient {
    pub fn new() -> (Self, async_channel::Receiver<Message>) {
        let (tx, rx) = async_channel::bounded(10);
        let inner = PendingRequests::new();
        let inner = Arc::new(Mutex::new(inner));
        let this = Self { inner, tx };
        (this, rx)
    }
    pub async fn send_request(
        &self,
        method: impl ToString,
        params: Option<impl Serialize>,
    ) -> Result<serde_json::Value, Error> {
        let method = method.to_string();
        let params = downcast_params(params)?;
        let (message, rx) = self.inner.lock().await.insert(method, params);
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

    pub async fn send_notification(
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

    pub async fn handle_response(&self, response: Response) {
        self.inner.lock().await.handle_response(response)
    }
}

pub struct PendingRequests {
    next_request_id: u32,
    pending_requests: HashMap<Id, oneshot::Sender<Response>>,
    // tx: async_channel::Sender<Message>,
}

impl PendingRequests {
    pub fn new() -> Self {
        Self {
            next_request_id: 1,
            pending_requests: Default::default(),
        }
    }
    pub fn insert(
        &mut self,
        method: String,
        params: Option<Params>,
    ) -> (Message, oneshot::Receiver<Response>) {
        let request_id = Id::Number(self.next_request_id);
        self.next_request_id += 1;
        let (tx, rx) = oneshot::channel();
        self.pending_requests.insert(request_id.clone(), tx);
        let request = Request {
            jsonrpc: Version::V2,
            method,
            params,
            id: Some(request_id),
        };
        let message = Message::Request(request);
        (message, rx)
    }
    pub fn handle_response(&mut self, response: Response) {
        if let Some(id) = &response.id {
            let tx = self.pending_requests.remove(id);
            if let Some(tx) = tx {
                let _ = tx.send(response);
            }
        }
    }
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

pub enum RpcSessionSink<T> {
    Idle(Option<RpcSession<T>>),
    Sending(Pin<Box<dyn Future<Output = RpcSession<T>> + 'static + Send>>),
}

impl<T> Sink<String> for RpcSessionSink<T>
where
    T: RpcServer + Unpin,
{
    type Error = io::Error;
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        match this {
            Self::Idle(_) => Poll::Ready(Ok(())),
            Self::Sending(fut) => match fut.as_mut().poll(cx) {
                Poll::Ready(session) => {
                    *this = Self::Idle(Some(session));
                    Poll::Ready(Ok(()))
                }
                Poll::Pending => Poll::Pending,
            },
        }
    }
    fn start_send(self: Pin<&mut Self>, item: String) -> Result<(), Self::Error> {
        let this = self.get_mut();
        match this {
            Self::Sending(_) => unreachable!(),
            Self::Idle(session) => {
                let session = session.take().unwrap();
                let fut = async move {
                    session.handle_incoming(&item).await;
                    session
                };
                let fut = Box::pin(fut);
                *this = Self::Sending(fut);
                Ok(())
            }
        }
    }
    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
