import { RawClient } from "./generated/client";
import { ChatMessage } from "./generated/types";
import { WebsocketTransport, Request } from "yerpc";

const USER_COLOR = '#'+(Math.random().toString(16)+'00000').slice(2,8)

window.addEventListener("DOMContentLoaded", (_event) => {
  run().catch(err => {
    console.error(err)
    document.getElementById("status")!.innerHTML = `Error: ${String(err)}`
  });
});
async function run() {
  const url = "ws://localhost:20808/rpc"
  const transport = new WebsocketTransport(url);
  const client = new RawClient(transport);

  const form = document.getElementById("form") as HTMLFormElement;
  form.onsubmit = async (ev) => {
    ev.preventDefault();
    const message = parseMessageFromForm(form);
    if (message) await client.send(message);
  };

  const updateStatus = (err?: Error) => {
    let status = `<strong>connected: ${transport.connected}</strong> <small>(url: ${transport.url}, reconnect attempts: ${transport.reconnectAttempts})</small>`
    if (err) status += `<div><strong>Error:</strong> ${String(err)}</div>`
    document.getElementById("status")!.innerHTML = status
  }

  transport.on("connect", updateStatus)
  transport.on("disconnect", updateStatus)
  transport.on("error", updateStatus)
  transport.on("request", (request: Request) => {
    const message = request.params as ChatMessage;
    appendMessageToLog(message);
  });

  const messages = await client.list();
  messages.forEach(appendMessageToLog);

}

function parseMessageFromForm(form: HTMLFormElement): null | ChatMessage {
  const data = new FormData(form);
  const content = data.get("content");
  const name = data.get("name");
  if (!content || !name) return null;
  return {
    content: content as string,
    user: {
      name: name as string,
      color: USER_COLOR
    },
  };
}

function appendMessageToLog(message: ChatMessage) {
  const el = document.createElement("li");
  el.innerHTML = `<strong style="color: ${message.user.color}">${message.user.name}:</strong> ${message.content}`;
  document.getElementById("log")!.prepend(el);
}
