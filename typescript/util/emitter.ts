// Adapted from https://github.com/scottcorgan/tiny-emitter
// (c) Scott Corgan
// License: MIT

export type Arguments<T> = [T] extends [(...args: infer U) => any] ? U : [T] extends [void] ? [] : [T];

export type EventsT = Record<string, (...args: any) => void>

type Callback = (...args: any[]) => void

type EventData = {
  callback: Callback
  ctx?: any
}

export class Emitter<T extends EventsT = any> {
  e: Map<keyof T, EventData[]>
  constructor () {
    this.e = new Map()
  }
  on<E extends keyof T>(event: E | string, callback: T[E] | Callback, ctx?: any) {
    return this._on(event, callback, ctx)
  }

  private _on<E extends keyof T>(event: E, callback: Callback, ctx?: any) {
    const data: EventData = { callback, ctx }
    if (!this.e.has(event)) this.e.set(event, [])
    this.e.get(event)!.push(data)
    return this;
  }

  once<E extends keyof T>(event: E, callback: T[E], ctx?: any) {
    const listener = (...args: any[]) => {
      this.off(event, callback)
      callback.apply(ctx, args)
    }
    this._on(event, listener, ctx)
  }

  // TODO: the any here is a temporary measure because I couldn't get the 
  // typescript inference right.
  emit<E extends keyof T>(event: E | string, ...args: Arguments<T[E]> | any[]) {
    if (!this.e.has(event)) return
    this.e.get(event)!.forEach(data => {
      data.callback.apply(data.ctx, args)
    })
    return this;
  }

  off<E extends keyof T>(event: E, callback?: T[E]) {
    if (!this.e.has(event)) return
    const existing = this.e.get(event)!
    const filtered = existing.filter(data => {
      return data.callback !== callback
    })
    if (filtered.length) {
      this.e.set(event, filtered)
    } else {
      this.e.delete(event)
    }
    return this
  }
}
