import { RawClient } from "./generated/client";
import { Request } from "./generated/jsonrpc";
import { ChatMessage } from "./generated/types";
import { WebsocketClient } from "jsonrpc";

window.addEventListener("DOMContentLoaded", (_event) => {
  run();
});
async function run() {
  const transport = new WebsocketClient("ws://localhost:20808/ws");
  const client = new RawClient(transport);

  transport.addEventListener("request", (event: Event) => {
    const request = (event as MessageEvent<Request>).data;
    const message = request.params as ChatMessage;
    appendMessageToLog(message);
  });

  const messages = await client.list();
  messages.forEach(appendMessageToLog);

  const form = document.getElementById("form") as HTMLFormElement;
  form.onsubmit = async (ev) => {
    ev.preventDefault();
    const message = parseMessageFromForm(form);
    if (message) await client.send(message);
  };
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
      color: "black",
    },
  };
}

function appendMessageToLog(message: ChatMessage) {
  const el = document.createElement("li");
  el.innerText = `${message.user.name}: ${message.content}`;
  document.getElementById("log")!.appendChild(el);
}
