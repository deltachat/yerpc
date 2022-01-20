import * as T from "./types.js"
import * as RPC from "./jsonrpc.js"

type RequestMethod = (method: string, params?: RPC.Params) => Promise<unknown>;
type NotificationMethod = (method: string, params?: RPC.Params) => void;
interface Transport {
  request: RequestMethod,
  notification: NotificationMethod
}

export class RawClient {
  private _request: RequestMethod;
  private _notification: NotificationMethod;

  constructor(transport: Transport) { this._request = transport.request.bind(transport); this._notification = transport.notification.bind(transport) }

      public send(message: T.ChatMessage): Promise<T.Usize> {
        return (this._request('send', [message] as RPC.Params)) as Promise<T.Usize>;
    }
    public list(): Promise<(T.ChatMessage)[]> {
        return (this._request('list', [] as RPC.Params)) as Promise<(T.ChatMessage)[]>;
    }

}
