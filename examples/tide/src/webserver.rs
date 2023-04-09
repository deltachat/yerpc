use async_std::sync::RwLock;
use async_std::task;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tide::Request;

use yerpc::typescript::TypeDef;
use yerpc::{rpc, RpcClient, RpcSession};

mod emitter;
use emitter::EventEmitter;

use tide_websockets::{Message as WsMessage, WebSocket};

#[derive(Serialize, Deserialize, TypeDef, Clone, Debug)]
struct User {
    name: String,
    color: String,
}

#[derive(Serialize, Deserialize, TypeDef, Clone, Debug)]
struct ChatMessage {
    content: String,
    user: User,
}

#[derive(Serialize, Deserialize, TypeDef, Clone, Debug)]
#[serde(tag = "type")]
enum Event {
    Message(ChatMessage),
    Joined(User),
}

#[derive(Clone)]
struct Backend {
    messages: Arc<RwLock<Vec<ChatMessage>>>,
    events: Arc<EventEmitter<(String, ChatMessage)>>,
}

impl Backend {
    pub fn new() -> Self {
        Self {
            messages: Default::default(),
            events: Arc::new(EventEmitter::new(10)),
        }
    }

    pub async fn post(&self, peer_addr: String, message: ChatMessage) -> anyhow::Result<usize> {
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

    pub async fn subscribe(&self) -> async_broadcast::Receiver<(String, ChatMessage)> {
        self.events.subscribe()
    }
}

#[derive(Clone)]
struct Session {
    peer_addr: String,
    backend: Backend,
    client: RpcClient,
}
impl Session {
    pub fn new(peer_addr: Option<&str>, backend: Backend, client: RpcClient) -> Self {
        let peer_addr = peer_addr.map(|addr| addr.to_string()).unwrap_or_default();
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
        task::spawn(async move {
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

#[rpc(ts_outdir = "typescript/generated")]
impl Session {
    /// Send a chat message.
    ///
    /// Pass the message to send.
    #[rpc(positional)]
    pub async fn send(&self, message: ChatMessage) -> yerpc::Result<usize> {
        let res = self.backend.post(self.peer_addr.clone(), message).await?;
        Ok(res)
    }

    /// List chat messages.
    #[rpc(positional)]
    pub async fn list(&self) -> yerpc::Result<Vec<ChatMessage>> {
        let list = self.backend.list().await;
        Ok(list)
    }
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    let backend = Backend::new();
    let mut app = tide::with_state(backend);

    app.at("/ws")
        .get(WebSocket::new(move |req: Request<Backend>, stream| {
            let backend = req.state().clone();
            let (client, mut out_rx) = RpcClient::new();
            let backend_session = Session::new(req.remote(), backend, client.clone());
            let session = RpcSession::new(client, backend_session);
            let stream_rx = stream.clone();
            task::spawn(async move {
                while let Some(message) = out_rx.next().await {
                    let message = serde_json::to_string(&message)?;
                    stream.send(WsMessage::Text(message)).await?;
                }
                let res: Result<(), anyhow::Error> = Ok(());
                res
            });
            async move {
                let sink = session.into_sink();
                stream_rx
                    .filter_map(|msg| async move {
                        match msg {
                            Ok(WsMessage::Text(input)) => Some(Ok(input)),
                            _ => None,
                        }
                    })
                    .forward(sink)
                    .await?;
                Ok(())
            }
        }));
    app.listen("127.0.0.1:20808").await?;

    Ok(())
}
