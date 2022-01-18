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

      public sum(params: T.SumParams): Promise<T.Usize> {
        return (this._request('sum', params as RPC.Params)) as Promise<T.Usize>;
    }
    public sum2(params: T.Sum2Params): Promise<T.Usize> {
        return (this._request('sum2', params as RPC.Params)) as Promise<T.Usize>;
    }
    public square(num: [T.F32]): Promise<T.F32> {
        return (this._request('square', num as RPC.Params)) as Promise<T.F32>;
    }
    public nothing(): Promise<null> {
        return (this._request('nothing', [] as RPC.Params)) as Promise<null>;
    }
    public manyArgs(a: T.Usize, b: (string)[]): Promise<null> {
        return (this._request('many_args', [a, b] as RPC.Params)) as Promise<null>;
    }
    public onevent(ev: unknown): void {
        return (this._notification('onevent', ev as RPC.Params)) as void;
    }
    public yell(message: string): Promise<string> {
        return (this._request('yell', [message] as RPC.Params)) as Promise<string>;
    }

}
