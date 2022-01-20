use async_std::sync::RwLock;
use async_std::task;
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tide::Request;

use yerpc::typescript::TypeDef;
use yerpc::{rpc, RpcHandle};
use yerpc_tide::yerpc_handler;

mod emitter;
use emitter::EventEmitter;

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
struct State {
    messages: Arc<RwLock<Vec<ChatMessage>>>,
    events: Arc<EventEmitter<(String, ChatMessage)>>,
}

impl State {
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
    state: State,
    client: RpcHandle,
}
impl Session {
    pub fn new(peer_addr: Option<&str>, state: State, client: RpcHandle) -> Self {
        let peer_addr = peer_addr.map(|addr| addr.to_string()).unwrap_or_default();
        let this = Self {
            peer_addr,
            state,
            client,
        };
        log::info!("Client connected: {}", this.peer_addr);
        this.spawn_event_loop();
        this
    }

    fn spawn_event_loop(&self) {
        let this = self.clone();
        task::spawn(async move {
            let mut message_events = this.state.subscribe().await;
            while let Some((_peer_addr, ev)) = message_events.next().await {
                // Optionally: This would be how to filter out messages that were emitted by ourselves.
                // if peer_addr != this.peer_addr {
                let res = this
                    .client
                    .notify("onevent", Some(Event::Message(ev)))
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
    #[rpc(positional)]
    pub async fn send(&self, message: ChatMessage) -> yerpc::Result<usize> {
        let res = self.state.post(self.peer_addr.clone(), message).await?;
        Ok(res)
    }

    #[rpc(positional)]
    pub async fn list(&self) -> yerpc::Result<Vec<ChatMessage>> {
        let list = self.state.list().await;
        Ok(list)
    }
}

#[async_std::main]
async fn main() -> Result<(), std::io::Error> {
    env_logger::init();
    let state = State::new();
    let mut app = tide::with_state(state);

    app.at("/ws")
        .get(yerpc_handler(|req: Request<State>, rpc| async move {
            Ok(Session::new(req.remote(), req.state().clone(), rpc))
        }));
    app.listen("127.0.0.1:20808").await?;

    Ok(())
}
