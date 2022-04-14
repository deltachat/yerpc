import WebSocket from 'isomorphic-ws'
import { TinyEmitter } from 'tiny-emitter';
import { Request, Response, Message, Error, Params } from './jsonrpc'

export interface Transport {
  request: (method: string, params?: Params) => Promise<unknown>,
  notification: (method: string, params?: Params) => void
}

type RequestMap = Map<
  number,
  { resolve: (result: unknown) => void; reject: (error: Error) => void }
>;

type Options = {
  reconnectDecay: number;
  reconnectInterval: number;
  maxReconnectInterval: number;
};

// type RequestEvent = WebSocket.MessageEvent<Request>;

export type ClientHandlerEvents = {
  "request": (request: Request) => void,
  [key:string]: (...ev:any) =>void
}
export abstract class ClientHandler extends TinyEmitter<ClientHandlerEvents> implements Transport {
  private _requests: RequestMap = new Map();
  private _requestId = 0;
  _send(_message: Message): void {
    throw new Error("_send method not implemented")
  }

  protected _onmessage(message: Message): void {
    if ((message as Request).method) {
      this.emit("request", message as Request);
    }

    if (!message.id) return; // TODO: Handle error;
    const response = message as Response;
    if (!response.id) return; // TODO: Handle error.
    const handler = this._requests.get(response.id);
    if (!handler) return; // TODO: Handle error.
    if (response.error) handler.reject(response.error);
    else handler.resolve(response.result);
  }

  notification(method: string, params?: Params): void {
    const request: Request = {
      jsonrpc: "2.0",
      method,
      id: 0,
      params,
    };
    this._send(request);
  }

  request(method: string, params?: Params): Promise<unknown> {
    // console.log('request', { method, params }, 'this', this)
    const id: number = ++this._requestId;
    const request: Request = {
      jsonrpc: "2.0",
      method,
      id,
      params,
    };
    this._send(request as Message);
    return new Promise((resolve, reject) => {
      this._requests.set(id, { resolve, reject });
    });
  }
}

export class WebsocketClient extends ClientHandler {
  _socket: ReconnectingWebsocket;
  constructor(public url: string, options?: Options) {
    super();
    const onmessage = (event: WebSocket.MessageEvent) => {
      const message: Message = JSON.parse(event.data as string);
      this._onmessage(message);
    };
    this._socket = new ReconnectingWebsocket(url, onmessage, options);
  }
  _send(message: Message): void {
    const serialized = JSON.stringify(message);
    this._socket.send(serialized);
  }
}

class ReconnectingWebsocket {
  socket!: WebSocket;
  ready!: Promise<void>;
  options: Options;

  private preopenQueue: string[] = [];
  private _connected = false;
  private reconnectAttempts = 0;

  onmessage: (event: WebSocket.MessageEvent) => void;
  closed = false;

  constructor(
    public url: string,
    onmessage: (event: WebSocket.MessageEvent) => void,
    options?: Options,
  ) {
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
    let resolveReady!: ((_: void) => void);
    this.ready = new Promise((resolve) => (resolveReady = resolve));

    this.socket = new WebSocket(this.url);
    this.socket.onmessage = this.onmessage.bind(this);
    this.socket.onopen = (_event) => {
      this.reconnectAttempts = 0;
      this._connected = true;
      while (this.preopenQueue.length) {
        this.socket.send(this.preopenQueue.shift() as string);
      }
      resolveReady();
    };

    this.socket.onclose = (_event) => {
      this._connected = false;
      const wait = Math.min(
        this.options.reconnectInterval *
          Math.pow(
            this.options.reconnectDecay,
            this.reconnectAttempts,
          ),
        this.options.maxReconnectInterval,
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
