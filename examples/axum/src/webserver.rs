use axum::{
    extract::{ws::WebSocketUpgrade, ConnectInfo},
    response::Response,
    routing::get,
    Extension, Router,
};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use schemars::JsonSchema;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;
use yerpc::axum::handle_ws_rpc;
use yerpc::typescript::TypeDef;
use yerpc::{rpc, OutReceiver, RpcClient, RpcSession};

mod emitter;
use emitter::EventEmitter;

#[derive(Serialize, Deserialize, TypeDef, JsonSchema, Clone, Debug)]
struct User {
    name: String,
    color: String,
}

#[derive(Serialize, Deserialize, TypeDef, JsonSchema, Clone, Debug)]
struct ChatMessage {
    content: String,
    user: User,
}

#[derive(Serialize, Deserialize, TypeDef, JsonSchema, Clone, Debug)]
#[serde(tag = "type")]
enum Event {
    Message(ChatMessage),
    Joined(User),
}

#[derive(Clone)]
struct Backend {
    messages: Arc<RwLock<Vec<ChatMessage>>>,
    events: Arc<EventEmitter<(SocketAddr, ChatMessage)>>,
}

impl Backend {
    pub fn new() -> Self {
        Self {
            messages: Default::default(),
            events: Arc::new(EventEmitter::new(10)),
        }
    }

    pub async fn post(&self, peer_addr: SocketAddr, message: ChatMessage) -> anyhow::Result<usize> {
        let len = {
            let mut messages = self.messages.write().await;
            messages.push(message.clone());
            messages.len()
        };
        self.events.emit((peer_addr, message)).await?;
        Ok(len)
    }

    pub async fn list(&self) -> Vec<ChatMessage> {
        self.messages.read().await.clone()
    }

    pub async fn subscribe(&self) -> async_broadcast::Receiver<(SocketAddr, ChatMessage)> {
        self.events.subscribe()
    }

    pub fn session(&self, peer_addr: SocketAddr) -> (RpcSession<Session>, OutReceiver) {
        let (client, out_receiver) = RpcClient::new();
        let backend_session = Session::new(peer_addr, self.clone(), client.clone());
        let session = RpcSession::new(client, backend_session);
        (session, out_receiver)
    }
}

#[derive(Clone)]
struct Session {
    peer_addr: SocketAddr,
    backend: Backend,
    client: RpcClient,
}
impl Session {
    pub fn new(peer_addr: SocketAddr, backend: Backend, client: RpcClient) -> Self {
        let this = Self {
            peer_addr,
            backend,
            client,
        };
        log::info!("Client connected: {}", this.peer_addr);
        this.spawn_event_loop();
        this
    }

    fn spawn_event_loop(&self) {
        let this = self.clone();
        tokio::spawn(async move {
            let mut message_events = this.backend.subscribe().await;
            while let Some((_peer_addr, ev)) = message_events.next().await {
                // Optionally: This would be how to filter out messages that were emitted by ourselves.
                // if peer_addr != this.peer_addr {
                let res = this
                    .client
                    .send_notification("onevent", Some(Event::Message(ev)))
                    .await;
                if res.is_err() {
                    break;
                }
                // }
            }
        });
    }
}

#[rpc]
impl Session {
    /// Send a chat message.
    ///
    /// Pass the message to send.
    #[rpc(positional)]
    pub async fn send(&self, message: ChatMessage) -> yerpc::Result<usize> {
        let res = self.backend.post(self.peer_addr, message).await?;
        Ok(res)
    }

    /// List chat messages.
    #[rpc(positional)]
    pub async fn list(&self) -> yerpc::Result<Vec<ChatMessage>> {
        let list = self.backend.list().await;
        Ok(list)
    }
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    tracing_subscriber::fmt::init();
    let backend = Backend::new();
    let app = Router::new()
        .route("/rpc", get(handler))
        .layer(TraceLayer::new_for_http())
        .layer(Extension(backend));
    let addr = SocketAddr::from(([127, 0, 0, 1], 20808));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();

    Ok(())
}

async fn handler(
    ws: WebSocketUpgrade,
    Extension(backend): Extension<Backend>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    let (session, out_channel) = backend.session(addr);
    handle_ws_rpc(ws, out_channel, session).await
}
