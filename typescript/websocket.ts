import WebSocket from "isomorphic-ws";
import { Request, Message, Error, Params } from "./jsonrpc.js";
import { BaseTransport } from "./client.js";
import { Emitter, EventsT } from "./util/emitter.js";

type WebsocketOptions = {
  reconnectDecay: number;
  reconnectInterval: number;
  maxReconnectInterval: number;
};

export interface WebsocketEvents extends EventsT {
  connect: () => void;
  disconnect: () => void;
  error: (error: Error) => void;
}

export class WebsocketTransport extends BaseTransport {
  _socket: ReconnectingWebsocket;
  constructor(public url: string, options?: WebsocketOptions) {
    super();
    const onmessage = (event: WebSocket.MessageEvent) => {
      const message: Message = JSON.parse(event.data as string);
      this._onmessage(message);
    };
    this._socket = new ReconnectingWebsocket(url, onmessage, options);

    this._socket.on("connect", () => this.emit("connect"));
    this._socket.on("disconnect", () => this.emit("disconnect"));
    this._socket.on("error", (error: WebSocket.ErrorEvent) =>
      this.emit("error", error)
    );
  }

  _send(message: Message): void {
    const serialized = JSON.stringify(message);
    this._socket.send(serialized);
  }
}

class ReconnectingWebsocket extends Emitter<WebsocketEvents> {
  socket!: WebSocket;
  ready!: Promise<void>;
  options: WebsocketOptions;

  private preopenQueue: string[] = [];
  private _connected = false;
  private reconnectAttempts = 0;

  onmessage: (event: WebSocket.MessageEvent) => void;
  closed = false;

  constructor(
    public url: string,
    onmessage: (event: WebSocket.MessageEvent) => void,
    options?: WebsocketOptions
  ) {
    super();
    this.options = {
      reconnectDecay: 1.5,
      reconnectInterval: 1000,
      maxReconnectInterval: 10000,
      ...options,
    };
    this.onmessage = onmessage;
    this._reconnect();
  }

  private _reconnect() {
    if (this.closed) return;
    let resolveReady!: (_: void) => void;
    this.ready = new Promise((resolve) => (resolveReady = resolve));

    this.socket = new WebSocket(this.url);
    this.socket.onmessage = this.onmessage.bind(this);
    this.socket.onopen = (_event) => {
      this.emit("connect");
      this.reconnectAttempts = 0;
      this._connected = true;
      while (this.preopenQueue.length) {
        this.socket.send(this.preopenQueue.shift() as string);
      }
      resolveReady();
    };
    this.socket.onerror = (error) => {
      this.emit("error", error);
    };

    this.socket.onclose = (_event) => {
      this._connected = false;
      this.emit("disconnect");
      const wait = Math.min(
        this.options.reconnectInterval *
          Math.pow(this.options.reconnectDecay, this.reconnectAttempts),
        this.options.maxReconnectInterval
      );
      setTimeout(() => {
        this.reconnectAttempts += 1;
        this._reconnect();
      }, wait);
    };
  }

  get connected(): boolean {
    return this._connected;
  }

  send(message: string): void {
    if (this.connected) this.socket.send(message);
    else this.preopenQueue.push(message);
  }

  close(): void {
    this.closed = true;
    this.socket.close();
  }
}
