import WebSocket from 'isomorphic-ws'
import type { MessageEvent } from 'ws'
import { Request, Response, Message, Error as jsonrpc_Error, Params, I32 } from './jsonrpc'

interface Transport {
  request: (method: string, params?: Params) => Promise<unknown>,
  notification: (method: string, params?: Params) => void
}

type RequestMap = Map<
  number,
  { resolve: (result: unknown) => void; reject: (error: jsonrpc_Error) => void, error_stack: string|undefined }
>;

type Options = {
  reconnectDecay: number;
  reconnectInterval: number;
  maxReconnectInterval: number;
};

class ClientError extends Error {
  code: I32
  data: jsonrpc_Error["data"]
  constructor(error: jsonrpc_Error){
    super(error.message)
    this.code = error.code
    this.data = error.data
  }
}

// type RequestEvent = MessageEvent<Request>;

function getStack() {
  // better way would be to use error-stack-parser for this
  const stack = new Error().stack
  if(!stack) {
    return undefined
  }
  if (stack.startsWith("Error\n")) {
    const frames = stack.split("\n");
    return ["[[ERROR]]", ...frames.slice(5)].join('\n');
  }
  else {
      return stack.split("\n").slice(3).join('\n');
  }
}

export abstract class ClientHandler extends EventTarget implements Transport {
  private _requests: RequestMap = new Map();
  private _requestId = 0;
  _send(_message: Message): void {
    throw new Error("_send method not implemented")
  }

  protected _onmessage(message: Message): void {
    if ((message as Request).method) {
      const request = message as Request;
      const event = new MessageEvent("request", { data: request });
      this.dispatchEvent(event);
    }

    if (!message.id) return; // TODO: Handle error;
    const response = message as Response;
    if (!response.id) return; // TODO: Handle error.
    const handler = this._requests.get(response.id);
    if (!handler) return; // TODO: Handle error.
    if (response.error) {
      const error = new ClientError(response.error)
      error.stack = handler.error_stack?.replace("[[ERROR]]", `Error: ${error.message}`)
      handler.reject(error)
    } else handler.resolve(response.result);
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
      this._requests.set(id, { resolve, reject, error_stack: getStack() });
    });
  }
}

export class WebsocketClient extends ClientHandler {
  _socket: ReconnectingWebsocket;
  constructor(public url: string, options?: Options) {
    super();
    const onmessage = (event: MessageEvent) => {
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

  onmessage: (event: MessageEvent) => void;
  closed = false;

  constructor(
    public url: string,
    onmessage: (event: MessageEvent) => void,
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
