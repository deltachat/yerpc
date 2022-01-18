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

  #methods
}
