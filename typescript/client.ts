import { Emitter } from "./util/emitter";
import { Request, Response, Message, Error, Params } from "./jsonrpc";

export interface Transport {
  request: (method: string, params?: Params) => Promise<unknown>;
  notification: (method: string, params?: Params) => void;
}

type RequestMap = Map<
  number,
  { resolve: (result: unknown) => void; reject: (error: Error) => void }
>;

type ClientEvents = {
  request: (request: Request) => void;
};

export abstract class ClientHandler
  extends Emitter<ClientEvents>
  implements Transport
{
  private _requests: RequestMap = new Map();
  private _requestId = 0;
  _send(_message: Message): void {
    throw new Error("_send method not implemented");
  }

  protected _onmessage(message: Message): void {
    if ((message as Request).method) {
      const request = message as Request;
      this.emit("request", request);
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
