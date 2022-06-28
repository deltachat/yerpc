import { RawClient } from "./generated/client";
import { ChatMessage } from "./generated/types";
import { WebsocketTransport, Request } from "yerpc";

window.addEventListener("DOMContentLoaded", (_event) => {
  run();
});
async function run() {
  const transport = new WebsocketTransport("ws://localhost:20808/ws");
  const client = new RawClient(transport);

  transport.on("connect", () => {
    document.getElementById("status")!.innerHTML = "connected!"
  })
  transport.on("disconnect", () => {
    document.getElementById("status")!.innerHTML = "disconnected!"
  })
  transport.on("error", (err: Error) => {
    document.getElementById("status")!.innerHTML = `Error: ${String(err)}`
  })
  transport.on("request", (request: Request) => {
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
